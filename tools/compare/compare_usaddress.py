"""Compare this parser with the Python usaddress library, token by token.

Both parsers label the same input tokens with usaddress's labels: ours
through `doorplate parse --usaddress` (ParsedAddress::to_usaddress_tags), theirs
through `usaddress.parse`. Run with usaddress available, for example:

    cargo build --release -p doorplate-cli
    uv run --with usaddress python tools/compare/compare_usaddress.py \
        --out report.md gold tests/external/usaddress/*.xml

    uv run --with usaddress python tools/compare/compare_usaddress.py \
        --out report.md raw data.csv --column address \
        [--line2 C --city C --state C --zip C] [--no-header] [--limit N]

    uv run --with usaddress python tools/compare/compare_usaddress.py \
        --out tag-report.md tag tests/external/usaddress/*.xml

`gold` scores both against usaddress's own labeled test files (held out from
its training data; see tests/external/usaddress/README.md). `raw` has no
labels: it reports where the two parsers disagree. `tag` compares the joined
dicts of `ParsedAddress::tag()` (`doorplate parse --tag`) with `usaddress.tag()` on
the strings of the same files, and the address types. The scoring rules are
in docs/usaddress-comparison.md.
"""
import argparse
import collections
import csv
import json
import os
import subprocess
import sys
import xml.etree.ElementTree as ET

# Gold labels for things this parser does not model (design non-goals:
# recipients, landmarks, intersections). Addresses that use them are scored
# apart. BuildingName is in scope since ParsedAddress::building.
OUT_OF_SCOPE = {
    "Recipient", "LandmarkName", "CornerOf",
    "IntersectionSeparator", "NotAddress", "CountryName", "SubaddressType",
    "SubaddressIdentifier", "StreetNamePreModifier", "AddressNumberPrefix",
}
# Labels that differ by convention only; both sides are mapped before
# comparing.
SAME = {"AddressNumberSuffix": "AddressNumber"}


def norm(label):
    return SAME.get(label, label) if label else None


def scored(text):
    return any(c.isalnum() for c in text)


def byte_spans(s, pieces):
    """Byte (start, end) of each piece, found in order in s."""
    out, pos = [], 0
    for p in pieces:
        i = s.find(p, pos)
        if i < 0:
            out.append(None)
            continue
        out.append((len(s[:i].encode()), len(s[: i + len(p)].encode())))
        pos = i + len(p)
    return out


def overlap_label(span, predicted):
    """Label of the first predicted (start, end, label) overlapping span."""
    if span is None:
        return None
    for a, b, label in predicted:
        if a < span[1] and span[0] < b:
            return label
    return None


def ours(strings, binary):
    """[(start, end, label)] per string, from `doorplate parse --usaddress`."""
    text = "\n".join(s.replace("\n", " ") for s in strings) + "\n"
    run = subprocess.run(
        [binary, "parse", "--usaddress"], input=text.encode(),
        capture_output=True, check=False,
    )
    lines = run.stdout.decode().splitlines()
    if len(lines) != len(strings):
        sys.exit(f"addr returned {len(lines)} records for {len(strings)} inputs")
    return [[(t[2], t[3], t[1]) for t in json.loads(l)["tags"]] for l in lines]


def theirs(strings):
    import usaddress  # noqa: PLC0415 (only needed when running)

    out = []
    for s in strings:
        try:
            parsed = usaddress.parse(s)
        except Exception:  # noqa: BLE001 (usaddress can raise on odd input)
            parsed = []
        spans = byte_spans(s, [t for t, _ in parsed])
        out.append([(sp[0], sp[1], l) for sp, (_, l) in zip(spans, parsed) if sp])
    return out


def ours_tags(strings, binary):
    """[record or None] per string, from `doorplate parse --tag`."""
    text = "\n".join(s.replace("\n", " ") for s in strings) + "\n"
    run = subprocess.run(
        [binary, "parse", "--tag"], input=text.encode(), capture_output=True, check=False
    )
    lines = run.stdout.decode().splitlines()
    if len(lines) != len(strings):
        sys.exit(f"addr returned {len(lines)} records for {len(strings)} inputs")
    return [json.loads(l) if json.loads(l)["tag"] is not None else None for l in lines]


def theirs_tags(strings):
    """[(dict, type) or None for RepeatedLabelError, or "error"] per string."""
    import usaddress  # noqa: PLC0415

    out = []
    for s in strings:
        try:
            out.append(usaddress.tag(s))
        except usaddress.RepeatedLabelError:
            out.append(None)
        except Exception:  # noqa: BLE001
            out.append("error")
    return out


def load_gold(path):
    """[(string, [(token, label)])] from a usaddress training-format file."""
    rows = []
    for el in ET.parse(path).getroot().iter("AddressString"):
        s, toks = el.text or "", []
        for child in el:
            toks.append((child.text or "", child.tag))
            s += (child.text or "") + (child.tail or "")
        rows.append((s, toks))
    return rows


def pct(n, d):
    return f"{100 * n / d:.1f}%" if d else "-"


# The street parts, collapsed for the coarse score. us50_test_tagged.xml
# labels a whole street as StreetName (`Mendenhall Mall Road,`), so only the
# coarse score is meaningful there.
STREET_PARTS = {
    "StreetNamePreDirectional", "StreetNamePreType", "StreetName",
    "StreetNamePostType", "StreetNamePostDirectional",
}


def coarse(label):
    return "StreetName" if label in STREET_PARTS else label


def key(s):
    return " ".join(s.split()).lower()


def load_seen(files, path):
    """Addresses usaddress saw in training (tests/external/usaddress/in_training.txt)."""
    path = path or os.path.join(os.path.dirname(os.path.abspath(files[0])), "in_training.txt")
    if not os.path.exists(path):
        return set()
    with open(path, encoding="utf-8") as fh:
        return {l.rstrip("\n") for l in fh if l.strip() and not l.startswith("#")}


def score(toks, spans, pred, fold):
    """(tokens, correct, [(gold, predicted)] errors) for one address."""
    n, ok, errors = 0, 0, []
    for (tok, gl), sp in zip(toks, spans):
        if not scored(tok):
            continue
        g, p = fold(norm(gl)), fold(norm(overlap_label(sp, pred)))
        n += 1
        if p == g:
            ok += 1
        else:
            errors.append((g, p))
    return n, ok, errors


def gold_cmd(args):
    seen = load_seen(args.files, args.training_overlap)
    folds = {"fine": lambda l: l, "coarse": coarse}
    totals = {g: collections.Counter() for g in ("held_out", "seen")}
    per_label = collections.defaultdict(collections.Counter)
    confusion = {"ours": collections.Counter(), "usaddress": collections.Counter()}
    example = {}
    only_theirs, only_ours = [], []
    file_rows = []
    for path in args.files:
        gold = load_gold(path)
        strings = [s for s, _ in gold]
        mine, other = ours(strings, args.addr), theirs(strings)
        f = {g: collections.Counter() for g in ("held_out", "seen")}
        counts = collections.Counter()
        for (s, toks), mp, tp in zip(gold, mine, other):
            counts["addresses"] += 1
            if any(l in OUT_OF_SCOPE for _, l in toks):
                counts["out_of_scope"] += 1
                continue
            group = "seen" if key(s) in seen else "held_out"
            counts[group] += 1
            spans = byte_spans(s, [t for t, _ in toks])
            right = {}
            for who, pred in (("ours", mp), ("usaddress", tp)):
                for fname, fold in folds.items():
                    n, ok, errors = score(toks, spans, pred, fold)
                    f[group][f"tokens_{fname}"] += n if who == "ours" else 0
                    f[group][f"{who}_{fname}_tok"] += ok
                    f[group][f"{who}_{fname}_addr"] += ok == n
                    if fname == "coarse":
                        right[who] = ok == n
                        if group == "held_out":
                            for g, p in errors:
                                confusion[who][(g, p)] += 1
                                example.setdefault((who, g, p), s)
            if group == "held_out":
                for (tok, gl), sp in zip(toks, spans):
                    if scored(tok):
                        g = coarse(norm(gl))
                        per_label[g]["n"] += 1
                        for who, pred in (("ours", mp), ("usaddress", tp)):
                            per_label[g][who] += coarse(norm(overlap_label(sp, pred))) == g
                if right["usaddress"] and not right["ours"]:
                    only_theirs.append(s)
                if right["ours"] and not right["usaddress"]:
                    only_ours.append(s)
        file_rows.append((os.path.basename(path), counts, f))
        for g in totals:
            totals[g].update(f[g])
    all_counts = sum((c for _, c, _ in file_rows), collections.Counter())

    def row(name, c, grp, n_addr):
        return (
            f"| {name} | {n_addr} | {grp['tokens_coarse']} "
            f"| {pct(grp['ours_coarse_tok'], grp['tokens_coarse'])} | {pct(grp['usaddress_coarse_tok'], grp['tokens_coarse'])} "
            f"| {pct(grp['ours_coarse_addr'], n_addr)} | {pct(grp['usaddress_coarse_addr'], n_addr)} "
            f"| {pct(grp['ours_fine_tok'], grp['tokens_fine'])} | {pct(grp['usaddress_fine_tok'], grp['tokens_fine'])} "
            f"| {pct(grp['ours_fine_addr'], n_addr)} | {pct(grp['usaddress_fine_addr'], n_addr)} |"
        )

    head = ("| File | Addresses | Tokens | Ours: tokens | usaddress: tokens | Ours: whole address "
            "| usaddress: whole address | Ours: tokens (fine) | usaddress: tokens (fine) "
            "| Ours: whole (fine) | usaddress: whole (fine) |")
    rule = "|---|---|---|---|---|---|---|---|---|---|---|"
    report = [
        "# usaddress comparison: labeled test files", "",
        "Scores are on the coarse labels (street parts folded into StreetName) unless marked fine.",
        "Rules: docs/usaddress-comparison.md.", "",
    ]
    if seen:
        report += [
            "**These files are usaddress's own regression tests** (its CI asserts these labels),",
            "so usaddress is expected to score near 100% on them, held out from training or not.",
            "Read the gap as where this parser differs from usaddress, not as an accuracy contest.", "",
        ]
    else:
        report += [
            "**Blind test:** no training-overlap list was found beside these files, so every",
            "address counts as unseen by both parsers.", "",
        ]
    report += [
        "| File | Addresses | Out of scope | In usaddress training files | Not in training files |",
        "|---|---|---|---|---|",
    ]
    for name, c, _ in file_rows + [("**All**", all_counts, None)]:
        report.append(f"| {name} | {c['addresses']} | {c['out_of_scope']} | {c['seen']} | {c['held_out']} |")
    report += ["", "## Not in usaddress's training files", "", head, rule]
    for name, c, f in file_rows:
        if c["held_out"]:
            report.append(row(name, c, f["held_out"], c["held_out"]))
    report.append(row("**All not in training**", all_counts, totals["held_out"], all_counts["held_out"]))
    if all_counts["seen"]:
        report += ["", "## Seen in usaddress's training data", "", head, rule]
        report.append(row("**All seen**", all_counts, totals["seen"], all_counts["seen"]))
    report += ["", "## By gold label (not in training files, coarse)", "",
               "| Label | Tokens | Ours | usaddress |", "|---|---|---|---|"]
    for label, c in sorted(per_label.items(), key=lambda kv: -kv[1]["n"]):
        report.append(f"| {label} | {c['n']} | {pct(c['ours'], c['n'])} | {pct(c['usaddress'], c['n'])} |")
    for who in ("ours", "usaddress"):
        report += ["", f"## Most common errors, not in training files: {who} (gold → predicted)", "",
                   "| Gold | Predicted | Tokens | Example |", "|---|---|---|---|"]
        for (g, p), n in confusion[who].most_common(args.top):
            report.append(f"| {g} | {p or '(unplaced)'} | {n} | `{example[(who, g, p)]}` |")
    report += ["", f"## Not in training files: right for usaddress, wrong for ours ({len(only_theirs)} addresses)", ""]
    report += [f"- `{s}`" for s in only_theirs[: args.top * 4]]
    report += ["", f"## Not in training files: right for ours, wrong for usaddress ({len(only_ours)} addresses)", ""]
    report += [f"- `{s}`" for s in only_ours[: args.top * 4]]
    return report


def raw_inputs(args):
    if args.file.endswith(".csv"):
        with open(args.file, newline="", encoding="utf-8", errors="replace") as fh:
            reader = csv.reader(fh)
            rows = list(reader)
        if args.no_header:
            col = {c: int(c) for c in filter(None, [args.column, args.line2, args.city, args.state, args.zip])}
        else:
            head, rows = rows[0], rows[1:]
            col = {c: head.index(c) for c in filter(None, [args.column, args.line2, args.city, args.state, args.zip])}
        get = lambda r, c: r[col[c]].strip() if c and col[c] < len(r) else ""  # noqa: E731
        out = []
        for r in rows:
            last = " ".join(x for x in (get(r, args.state), get(r, args.zip)) if x)
            parts = [get(r, args.column), get(r, args.line2), get(r, args.city), last]
            s = ", ".join(" ".join(p.split()) for p in parts if p)
            if s:
                out.append(s)
    else:
        with open(args.file, encoding="utf-8", errors="replace") as fh:
            out = [" ".join(l.split()) for l in fh if l.strip()]
    return out[: args.limit] if args.limit else out


# Labels usaddress uses where this parser uses another for the same thing.
RAW_SAME = {"SubaddressType": "OccupancyType", "SubaddressIdentifier": "OccupancyIdentifier"}


def raw_cmd(args):
    strings = raw_inputs(args)
    mine, other = ours(strings, args.addr), theirs(strings)
    c = collections.Counter()
    pairs, example = collections.Counter(), {}
    for s, mp, tp in zip(strings, mine, other):
        same_fine = same_coarse = True
        for a, b, mine_label in mp:
            if not scored(s.encode()[a:b].decode(errors="replace")):
                continue
            c["tokens"] += 1
            m = norm(mine_label)
            t = norm(overlap_label((a, b), tp))
            t = RAW_SAME.get(t, t)
            if m is None:
                c["unplaced"] += 1
            if m == t:
                c["same_fine"] += 1
            else:
                same_fine = False
                pairs[(m, t)] += 1
                example.setdefault((m, t), s)
            if coarse(m) == coarse(t):
                c["same_coarse"] += 1
            else:
                same_coarse = False
        c["addr_fine"] += same_fine
        c["addr_coarse"] += same_coarse
    n = len(strings)
    report = [
        f"# usaddress comparison: {os.path.basename(args.file)} (no labels)", "",
        "No gold labels: these are agreement rates, not accuracy. Rules: docs/usaddress-comparison.md.", "",
        f"- Addresses: {n}",
        f"- Same labels on every token: {c['addr_fine']} ({pct(c['addr_fine'], n)}); "
        f"coarse (street parts folded): {c['addr_coarse']} ({pct(c['addr_coarse'], n)})",
        f"- Tokens with the same label: {pct(c['same_fine'], c['tokens'])} of {c['tokens']}; "
        f"coarse: {pct(c['same_coarse'], c['tokens'])}",
        f"- Tokens this parser left unplaced (unparsed, placeholders, dropped copies): "
        f"{c['unplaced']} ({pct(c['unplaced'], c['tokens'])})",
        "", "## Disagreements (ours vs usaddress, fine labels)", "",
        "| Ours | usaddress | Tokens | Example |", "|---|---|---|---|",
    ]
    for (m, t), k in pairs.most_common(args.top):
        report.append(f"| {m or '(unplaced)'} | {t or '(none)'} | {k} | `{example[(m, t)]}` |")
    return report


def tag_cmd(args):
    seen = load_seen(args.files, args.training_overlap)
    strings, rows = [], []
    for path in args.files:
        for s, toks in load_gold(path):
            if any(l in OUT_OF_SCOPE for _, l in toks):
                continue
            strings.append(s)
    strings = list(dict.fromkeys(strings))
    mine, other = ours_tags(strings, args.addr), theirs_tags(strings)
    c = collections.Counter()
    label_diff = collections.defaultdict(collections.Counter)
    example = {}
    types = collections.Counter()
    for s, m, t in zip(strings, mine, other):
        c["addresses"] += 1
        c["seen" if key(s) in seen else "held_out"] += 1
        if m is None or t == "error":
            c["unusable"] += 1
            continue
        if t is None:
            c["usaddress_repeated_label_error"] += 1
            c["...and ours reported a repeated label"] += bool(m["repeated_labels"])
            continue
        theirs_dict, theirs_type = t
        if m["repeated_labels"]:
            c["ours_repeated_label_only"] += 1
        types[(m["usaddress_type"], theirs_type)] += 1
        same = True
        for label in sorted(set(m["tag"]) | set(theirs_dict)):
            a, b = m["tag"].get(label), theirs_dict.get(label)
            if a == b:
                continue
            same = False
            kind = "only ours" if b is None else "only usaddress" if a is None else "value differs"
            label_diff[label][kind] += 1
            example.setdefault((label, kind), (s, a, b))
        c["same_dict"] += same
        c["compared"] += 1
    n = c["compared"]
    report = [
        "# usaddress.tag() vs ParsedAddress::tag()", "",
        "Strings of usaddress's labeled test files, minus those that use a label this "
        "parser does not model. Rules: docs/usaddress-comparison.md.", "",
        f"- Addresses: {c['addresses']} ({c['held_out']} not in usaddress's training files, "
        f"{c['seen']} in them); unusable (no text, or usaddress failed): {c['unusable']}",
        f"- usaddress raised RepeatedLabelError: {c['usaddress_repeated_label_error']} "
        f"(ours reported a repeated label on {c['...and ours reported a repeated label']} of them); "
        f"ours reported one where usaddress did not: {c['ours_repeated_label_only']}",
        f"- Compared (neither failed): {n}; the whole dict is the same on {c['same_dict']} ({pct(c['same_dict'], n)})",
        "", "## Where the joined values differ (labels, by kind)", "",
        "| Label | Value differs | Only ours | Only usaddress | Example (ours / usaddress) |",
        "|---|---|---|---|---|",
    ]
    for label, k in sorted(label_diff.items(), key=lambda kv: -sum(kv[1].values()))[: args.top]:
        kind = max(k, key=k.get)
        s, a, b = example[(label, kind)]
        report.append(
            f"| {label} | {k['value differs']} | {k['only ours']} | {k['only usaddress']} "
            f"| `{s}`: `{a}` / `{b}` |"
        )
    report += ["", "## Address type (usaddress_type() / usaddress's)", "",
               "| Ours | usaddress | Addresses |", "|---|---|---|"]
    for (a, b), k in types.most_common():
        report.append(f"| {a} | {b} | {k} |")
    return report


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--addr", default="target/release/doorplate", help="path to the doorplate binary")
    ap.add_argument("--out", help="write the markdown report here (default: stdout)")
    ap.add_argument("--top", type=int, default=15, help="rows per error table")
    sub = ap.add_subparsers(dest="cmd", required=True)
    g = sub.add_parser("gold", help="score both parsers on usaddress-format XML")
    g.add_argument("files", nargs="+")
    g.add_argument("--training-overlap", help="list of addresses usaddress trained on (default: in_training.txt beside the files)")
    t = sub.add_parser("tag", help="compare ParsedAddress::tag() with usaddress.tag()")
    t.add_argument("files", nargs="+")
    t.add_argument("--training-overlap", help="as for gold")
    r = sub.add_parser("raw", help="compare both parsers on unlabeled input")
    r.add_argument("file", help=".csv, or text with one address per line")
    r.add_argument("--column")
    for opt in ("--line2", "--city", "--state", "--zip"):
        r.add_argument(opt)
    r.add_argument("--no-header", action="store_true", help="CSV has no header; columns are 0-based indexes")
    r.add_argument("--limit", type=int)
    args = ap.parse_args()
    if args.cmd == "raw" and args.file.endswith(".csv") and not args.column:
        ap.error("raw .csv input needs --column")
    report = {"gold": gold_cmd, "tag": tag_cmd, "raw": raw_cmd}[args.cmd](args)
    text = "\n".join(report) + "\n"
    if args.out:
        with open(args.out, "w", encoding="utf-8") as fh:
            fh.write(text)
    else:
        sys.stdout.write(text)


if __name__ == "__main__":
    main()
