"""Triage the blind Florida sample for collaborative labeling (item 36).

    python3 tools/calibrate/triage.py                 # report: clear and review rows
    python3 tools/calibrate/triage.py --accept ID...  # accept drafts as checked by the assistant
    python3 tools/calibrate/triage.py --apply ID...   # apply recommendations.ndjson proposals

The process is in `tools/calibrate/README.md` ("Collaborative mode"). This
script never trusts the parser: a draft is **clear** only when an independent
walk of the input accounts for every word, in address order, with the
draft's values, and nothing in it needs judgment. Everything else is
**review**, with the reasons. `--accept` drops `TODO` from unchanged drafts
and records `by=assistant` in the note; `--apply` writes a proposed label
from `recommendations.ndjson` (beside the sample) and records
`by=assistant+author`. Both refuse a row that is already labeled or that the
corpus runner rejects.

The report (`TRIAGE.md` beside the sample) holds private rows: it stays in
`local-data/`.
"""
import argparse
import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import label as L  # noqa: E402
import sample_fl  # noqa: E402

SAMPLE = sample_fl.ROOT / "local-data" / "florida-sample"
GROUPS = {
    "change": "Change: the draft is wrong (parser errors, go to NOTES.md)",
    "your call": "Your call: judgment needed; recommendation given",
    "ambiguous": "Accept as drafted, marked ambiguous",
    "accept": "Accept as drafted: correct by the conventions; held back only by a warning",
}
# Fields in the order they are written in an address, with the table that
# maps an input spelling to the stored value (None: compared as written).
ORDER = [
    ("street_number", None), ("street_dir_prefix", L.DIR), ("street_name", None),
    ("street_suffix", L.SUFFIX), ("street_dir_suffix", L.DIR),
    ("unit_type", L.UNIT), ("unit_number", None),
    ("second_unit_type", L.UNIT), ("second_unit_number", None),
    ("po_box", None), ("city", None), ("state", L.STATE), ("zip5", None), ("zip4", None),
]
WALKED = {f for f, _ in ORDER}
HIGHWAY_WORDS = {"HIGHWAY", "HWY", "SR", "CR", "US", "ROUTE", "RTE", "INTERSTATE", "COUNTY", "STATE"}
BOX_WORDS = {"PO", "P", "O", "BOX", "POST", "OFFICE", "BX", "POB", "DRAWER"}


def words(record):
    """The input's words as the walk reads them: `#` kept, ZIP+4 split."""
    out = []
    for w in L.words_of(record):
        if re.fullmatch(r"\d{5}-\d{4}", w):
            out += w.split("-")
        elif len(w) > 1 and w.startswith("#"):
            out += ["#", w[1:]]
        else:
            out.append(w)
    return out


def walk(record, expect):
    """Match the input words to the draft's fields in address order.
    Returns a list of problems (empty when every word is accounted for)."""
    ws = words(record)
    i = 0
    for field, table in ORDER:
        value = expect.get(field)
        if not value:
            continue
        if field == "po_box":
            while i < len(ws) and ws[i] in BOX_WORDS:
                i += 1
        if field.endswith("unit_number"):
            while i < len(ws) and ws[i] == "#":
                i += 1
        n = len(value.split())
        # A spelling may be one or two input words (`NORTH EAST`).
        for k in (n, n + 1, 1, 2):
            got = " ".join(ws[i : i + k])
            if (table.get(got) if table else got) == value:
                i += k
                break
        else:
            return [f"{field} {value!r} not found at input word {i + 1} ({' '.join(ws[i:i + 3])!r})"]
    if i < len(ws):
        return [f"input words not in any field: {' '.join(ws[i:])!r}"]
    return []


def reasons(record, expect, warnings, place):
    r = []
    if warnings:
        r.append("warnings: " + ", ".join(warnings))
    other = sorted(set(expect) - WALKED)
    if other:
        r.append("fields needing judgment: " + ", ".join(other))
    if not other:
        r += walk(record, expect)
    name = expect.get("street_name", "").split()
    # `GRACE LAKE CIR` is normal; `GRACE LAKE` with no suffix may have
    # swallowed it.
    if name and name[-1] in L.SUFFIX and not expect.get("street_suffix"):
        r.append(f"no suffix, and the name ends in a suffix word ({name[-1]})")
    # `750 NE 178ST` (a glued suffix), `4731 HIGHWAY A1A` (a highway read as
    # a name): a street with no suffix is checked by eye.
    if name and not expect.get("street_suffix"):
        r.append("street with no suffix")
    if any(w in HIGHWAY_WORDS for w in name):
        r.append("street name holds a highway word")
    if any(w in L.DIR for w in name):
        r.append("street name holds a directional word")
    if expect.get("unit_number") and not expect.get("unit_type"):
        r.append("unit with no designator")
    if expect.get("city"):
        if not place:
            r.append("city is not a known place")
        elif place.get("source") != "name":
            r.append(f"city matched by {place.get('source')} (spelling or completion)")
    if "fields" in record and expect.get("city"):
        if L.clean(record["fields"].get("city", "")) != expect["city"]:
            r.append("city differs from the city field")
    return r


def load():
    path = SAMPLE / "sample.jsonl"
    return path, [json.loads(l) for l in path.read_text().splitlines() if l.strip()]


def masked(record):
    """The input for the report; an attention line may hold a person's name."""
    return L.input_text(record)


def report():
    path, rows = load()
    exe = sample_fl.addr_bin()
    clear, review = [], []
    for rec in rows:
        if not rec.get("note", "").endswith("TODO"):
            continue
        d = sample_fl.parse(exe, rec)
        r = reasons(rec, rec["expect"], rec["warnings"], d.get("place"))
        (review if r else clear).append((rec, r))
    out = ["# Triage (private: stays in local-data/)", ""]
    out.append(f"{len(clear)} clear, {len(review)} to review, of {len(clear) + len(review)} unlabeled.")
    out += ["", "## Clear", "", "Every input word accounted for, in order; known city; no warnings.", ""]
    for rec, _ in clear:
        e = rec["expect"]
        out.append(f"- {rec['id']}: `{masked(rec)}` -> " + " | ".join(f"{k}={L.show(v)}" for k, v in e.items()))
    out += ["", "## Review", ""]
    for rec, r in review:
        e = rec["expect"]
        out.append(f"- {rec['id']}: `{masked(rec)}`")
        out.append("  - draft: " + " | ".join(f"{k}={L.show(v)}" for k, v in e.items()))
        out.append("  - why: " + "; ".join(r))
    (SAMPLE / "TRIAGE.md").write_text("\n".join(out) + "\n")
    print(f"{len(clear)} clear, {len(review)} to review; report in {SAMPLE / 'TRIAGE.md'}")
    review_file(rows)


def proposals():
    path = SAMPLE / "recommendations.ndjson"
    if not path.exists():
        return {}
    return {p["id"]: p for p in map(json.loads, path.read_text().splitlines()) if p}


def review_file(rows):
    """REVIEW.md: the recommendations for rows still unlabeled, by group."""
    recs = proposals()
    todo = {r["id"]: r for r in rows if r.get("note", "").endswith("TODO")}
    if not recs:
        return

    def fmt(e):
        return " | ".join(f"{k}={L.show(v)}" for k, v in e.items() if k != "state")

    out = [
        "# Review: recommendations for the rows triage could not clear", "",
        "Private: stays in local-data/. Process: tools/calibrate/README.md, \"Collaborative mode\".",
        "Approve with `python3 tools/calibrate/triage.py --apply ID...`; overrule one with",
        "`python3 tools/calibrate/label.py --start ID`. Applied rows drop off on the next run.", "",
    ]
    for g, title in GROUPS.items():
        ps = [p for p in recs.values() if p.get("group") == g and p["id"] in todo]
        out += [f"## {title} ({len(ps)})", ""]
        for p in sorted(ps, key=lambda p: p["id"]):
            r = todo[p["id"]]
            out.append(f"- **{p['id']}** `{masked(r)}`")
            if g in ("change", "your call"):
                out.append(f"  - draft: {fmt(r['expect'])}  warnings: {', '.join(r['warnings']) or 'none'}")
                out.append(f"  - **proposed:** {fmt(p['expect'])}  warnings: {', '.join(p['warnings']) or 'none'}")
            out.append("  - why: " + (f"ambiguous: {p['ambiguous']}" if g == "ambiguous" else p["why"]))
        out.append("")
    waiting = sorted(set(todo) - set(recs))
    if waiting:
        out += [f"## No recommendation yet ({len(waiting)})", "", ", ".join(waiting), ""]
    (SAMPLE / "REVIEW.md").write_text("\n".join(out))
    print(f"recommendations for {len(set(todo) & set(recs))} unlabeled rows in {SAMPLE / 'REVIEW.md'}")


def write(ids, proposals, by):
    path, rows = load()
    exe = sample_fl.addr_bin()
    s = L.Session.__new__(L.Session)
    s.exe = exe
    index = {r["id"]: n for n, r in enumerate(rows)}
    done = 0
    for rid in ids:
        if rid not in index:
            print(f"{rid}: no such row")
            continue
        rec = dict(rows[index[rid]])
        if not rec.get("note", "").endswith("TODO"):
            print(f"{rid}: already labeled; left alone")
            continue
        meta = [p for p in rec["note"].removesuffix("TODO").strip().rstrip(";").split("; ") if p]
        if proposals is not None:
            if rid not in proposals:
                print(f"{rid}: no recommendation; left alone")
                continue
            p = proposals[rid]
            rec["expect"] = L.ordered(p["expect"])
            rec["warnings"] = p["warnings"]
            if p.get("ambiguous"):
                meta.append(f"ambiguous: {p['ambiguous']}")
        rec["note"] = "; ".join(meta + [f"by={by}"])
        problem = s.valid(rec)
        if problem:
            print(f"{rid}: rejected by the corpus runner; left alone\n{problem}")
            continue
        rows[index[rid]] = rec
        done += 1
        if proposals is not None and proposals[rid].get("parser_error"):
            notes = SAMPLE / "NOTES.md"
            text = notes.read_text() if notes.exists() else "# Parser errors in the Florida sample\n\n"
            lines = [l for l in text.splitlines() if not l.startswith(f"{rid}: ")]
            lines.append(f"{rid}: {proposals[rid]['parser_error']}")
            head = [l for l in lines if not l.startswith("fs-")]
            body = sorted(l for l in lines if l.startswith("fs-"))
            notes.write_text("\n".join(head).rstrip() + "\n\n" + "\n".join(body) + "\n")
    tmp = path.with_suffix(".jsonl.tmp")
    tmp.write_text("".join(json.dumps(r) + "\n" for r in rows))
    tmp.replace(path)
    print(f"{done} row(s) written")


def main():
    global SAMPLE
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--accept", nargs="+", metavar="ID", help="accept these drafts unchanged")
    ap.add_argument("--apply", nargs="+", metavar="ID", help="apply these recommendations")
    ap.add_argument("--group", choices=list(GROUPS), help="with --apply ALL: only this group")
    ap.add_argument("--dir", type=Path, default=SAMPLE, help="the sample's directory")
    a = ap.parse_args()
    SAMPLE = a.dir
    if a.apply == ["ALL"]:
        a.apply = sorted(p["id"] for p in proposals().values() if not a.group or p.get("group") == a.group)
    if a.accept:
        write(a.accept, None, "assistant")
    elif a.apply:
        write(a.apply, proposals(), "assistant+author")
    else:
        report()


if __name__ == "__main__":
    main()
