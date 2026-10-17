"""Score this parser against the Netsyms sample (private data; output stays local).

Each US row of sample.sqlite (tools/netsyms/sample.py) is written back into
an address string in several styles, parsed with `doorplate`, and compared field
by field with the row's own columns:

    line     123 N MAIN ST APT 4 MIAMI FL 33101      one line, no commas
    commas   123 N MAIN ST APT 4, MIAMI, FL 33101
    mixed    123 N Main St Apt 4, Miami, FL 33101    commas, mixed case
    fields   line1 "123 N MAIN ST", line2 "APT 4", city, state, zip

    cargo build --release -p doorplate-cli
    python3 tools/netsyms/compare.py \
        --sample local-data/netsyms-2026/sample.sqlite \
        --out local-data/netsyms-2026/reports/run1.md

The source's labels are not hand-checked (docs/future-sources.md), so a
mismatch is a parser bug, a source error or a naming difference. The report
groups mismatches by what differs, and `--dump` writes them as JSON lines
for review. Headline rates use the uniform draw (unbiased); the other draws
(quota, unit, odd_number, highway) are stress sets and are reported apart.
Nothing here is a deliverability check.
"""
import argparse
import collections
import csv
import difflib
import io
import json
import re
import sqlite3
import subprocess
import sys
import tomllib
from pathlib import Path

STYLES = ["line", "commas", "mixed", "fields"]
CHECKS = ["number", "street", "unit", "city", "state", "zip"]


def clean(s):
    return re.sub(r"\s+", " ", re.sub(r"[.,]", " ", (s or "").upper())).strip()


NULLS = re.compile(r"\s*<NULL>\s*")


def strip_nulls(v):
    """The source writes a missing part as the text `<NULL>` inside a value."""
    return NULLS.sub(" ", v).strip() if v and "<NULL>" in v else v


# Spellings that differ by convention only are not mismatches: the Pub 28
# abbreviations against the spelled-out word the source kept. Built from the
# committed data tables, so both sides are reduced the same way.
DATA = Path(__file__).resolve().parents[2] / "crates" / "doorplate" / "data"


def _canon_table():
    table = {}
    for name, key in (("suffixes.toml", "suffix"), ("directionals.toml", "directional")):
        with open(DATA / name, "rb") as f:
            for row in tomllib.load(f)[key]:
                for form in row["forms"] + row.get("observed", []):
                    table[form.upper()] = row["abbr"]
    with open(DATA / "states.toml", "rb") as f:
        for row in tomllib.load(f)["state"]:
            if " " not in row["name"]:
                table[row["name"]] = row["code"]
    return table


CANON = _canon_table()
PHRASES = [  # whole-phrase conventions the word table cannot express
    (re.compile(r"\b(SR|STATE RD)\b"), "STATE RD"),
    (re.compile(r"\b(CR|CTY RD|CO RD|COUNTY RD)\b"), "COUNTY RD"),
    (re.compile(r"\bU S\b"), "US"),
    (re.compile(r"\bCO (HWY|RD)\b"), r"COUNTY \1"),
    (re.compile(r"\bRT\b"), "RTE"),
    (re.compile(r"\bI-?(\d)"), r"INTERSTATE \1"),
    (re.compile(r"\b([A-Z]{1,3})[-.](\d)"), r"\1 \2"),
    (re.compile(r"\b([A-Z]) (\d)"), r"\1\2"),
]


def drop_hwy(s):
    """`NV 31` and `NV HWY 31` are the same road; Pub 28 writes the word."""
    return re.sub(r"\bHWY\b ?", "", s).strip()


def canon(s):
    s = drop_hwy(" ".join(CANON.get(w, w) for w in s.split()))
    for rx, rep in PHRASES:
        s = rx.sub(rep, s)
    return s


DESIGNATORS = {
    "#", "APT", "APARTMENT", "UNIT", "STE", "SUITE", "BLDG", "BUILDING", "FL", "FLOOR", "RM",
    "ROOM", "LOT", "SPC", "SPACE", "TRLR", "TRAILER", "DEPT", "PH", "OFC", "OFFICE", "STUDIO",
    "SLIP", "HNGR", "HANGAR", "PIER", "KEY", "LBBY", "LOBBY", "LOWR", "LOWER", "UPPR", "UPPER",
    "REAR", "FRNT", "FRONT", "SIDE", "BSMT", "BASEMENT", "BAY", "BOX", "STOP",
}
UNIT_VALUE = re.compile(r"^[A-Z0-9][A-Z0-9-]{0,7}$")
NUMBER_OK = re.compile(r"^[0-9]+[A-Z]?(-[0-9]+)?$|^[A-Z][0-9]+$|^[0-9]+ ?[0-9]/[0-9]$")


def quirks(row):
    """Why this source row is not a fair test, or ''. Real input would not
    be written this way: the source's own fields are malformed."""
    number, street, street2, _, _, _ = row
    why = []
    if not NUMBER_OK.match(number.strip().upper()) or not number.strip("0"):
        why.append("number")  # malformed, or all zeros (no number; zero_street_number)
    if street2:
        toks = clean(street2).replace("#", " # ").split()
        if not (len(toks) == 2 and toks[0] in DESIGNATORS and UNIT_VALUE.match(toks[1])):
            why.append("unit")
    if re.match(r"^\d+/\d+ ", street):
        why.append("street")  # a fraction that belongs to the number
    if re.search(r"[(),<>]", street) or re.search(r"\d\s+\d", street) and not re.search(r"\d+/\d", street):
        why.append("street")
    return "+".join(why)


def first_diff(a, b):
    """The first differing token run between two strings, as (expected, ours)."""
    x, y = a.split(), b.split()
    for op, i1, i2, j1, j2 in difflib.SequenceMatcher(None, x, y, autojunk=False).get_opcodes():
        if op != "equal":
            return " ".join(x[i1:i2]) or "(none)", " ".join(y[j1:j2]) or "(none)"
    return "", ""


def title(s):
    out = []
    for w in s.split():
        out.append(w.lower() if any(c.isdigit() for c in w) else w.capitalize())
    return " ".join(out)


def render(row, style):
    number, street, street2, city, state, zipcode = row
    unit = f" {street2}" if street2 else ""
    if style == "line":
        return f"{number} {street}{unit} {city} {state} {zipcode}"
    if style == "commas":
        return f"{number} {street}{unit}, {city}, {state} {zipcode}"
    if style == "mixed":
        return title(f"{number} {street}{unit}") + f", {title(city)}, {state} {zipcode}"
    raise ValueError(style)


def unit_value(street2):
    """The value part of a source unit: `APT 4` -> `4`, `# 9` -> `9`."""
    toks = clean(street2).replace("#", " ").split()
    return toks[-1] if toks else ""


def street_text(c):
    """The street as a string from our components, in the source's order."""
    if c.get("highway"):
        parts = [
            c.get("street_dir_prefix"),
            c["highway"],
            c.get("street_name"),
            c.get("street_suffix"),
            c.get("street_dir_suffix"),
        ]
    else:
        parts = [
            c.get("street_dir_prefix"),
            c.get("street_name"),
            c.get("street_suffix"),
            c.get("street_dir_suffix"),
        ]
    return clean(" ".join(p for p in parts if p))


def ours_of(c):
    return {
        "number": clean(c.get("street_number")).replace(" ", ""),
        "street": street_text(c),
        "unit": clean(c.get("unit_number")),
        "city": clean(c.get("city")),
        "state": clean(c.get("state")),
        "zip": clean(c.get("zip5")),
    }


def expected_of(row):
    number, street, street2, city, state, zipcode = row
    return {
        "number": clean(number).replace(" ", ""),
        "street": clean(street),
        "unit": unit_value(street2) if street2 else "",
        "city": clean(city),
        "state": clean(state),
        "zip": clean(zipcode)[:5],
    }


def run_lines(binary, strings):
    out = []
    for i in range(0, len(strings), 100_000):
        chunk = [s.replace("\n", " ") for s in strings[i : i + 100_000]]
        run = subprocess.run(
            [binary, "parse", "-"], input="\n".join(chunk), capture_output=True, text=True, check=True
        )
        lines = run.stdout.splitlines()
        if len(lines) != len(chunk):
            sys.exit(f"addr returned {len(lines)} records for {len(chunk)} inputs")
        out += [json.loads(line) for line in lines]
    return out


def run_fields(binary, rows, tmp):
    path = f"{tmp}/fields.csv"
    with open(path, "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["line1", "line2", "city", "state", "zip"])
        for number, street, street2, city, state, zipcode in rows:
            w.writerow([f"{number} {street}", street2 or "", city, state, zipcode])
    run = subprocess.run(
        [binary, "parse-file", path, "--column", "line1", "--line2", "line2", "--city", "city",
         "--state", "state", "--zip", "zip"],
        capture_output=True, text=True, check=True,
    )
    recs = []
    for r in csv.DictReader(io.StringIO(run.stdout)):
        recs.append(
            (
                {
                    "street_number": r["addr_street_number"],
                    "street_dir_prefix": r["addr_street_dir_prefix"],
                    "street_name": r["addr_street_name"],
                    "street_suffix": r["addr_street_suffix"],
                    "street_dir_suffix": r["addr_street_dir_suffix"],
                    "highway": r["addr_highway"],
                    "unit_number": r["addr_unit_number"],
                    "city": r["addr_city"],
                    "state": r["addr_state"],
                    "zip5": r["addr_zip5"],
                },
                [w for w in r["addr_warnings"].split(";") if w],
                r["addr_unparsed"],
            )
        )
    if len(recs) != len(rows):
        sys.exit("parse-file row count differs")
    return recs


def pct(n, d):
    return f"{100 * n / d:.2f}%" if d else "-"


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--sample", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--addr", default="target/release/doorplate")
    ap.add_argument("--dump", help="write mismatches (JSON lines) here")
    ap.add_argument("--limit", type=int, help="rows per draw, for a quick run")
    ap.add_argument("--tmp", default="/tmp")
    ap.add_argument("--styles", default=",".join(STYLES))
    args = ap.parse_args()
    styles = args.styles.split(",")

    db = sqlite3.connect(f"file:{args.sample}?mode=ro", uri=True)
    rows = []  # (why, source, row)
    nulls = collections.Counter()  # rows with a literal <NULL> in a value, by draw
    for why in ["uniform", "quota", "unit", "odd_number", "highway"]:
        q = (
            "select number, street, street2, city, state, zipcode, source from sample "
            "where country = 'US' and why = ? and number not like '-%' and number <> ''"
        )
        if args.limit:
            q += f" limit {args.limit}"
        for r in db.execute(q, (why,)):
            if any(v and "<NULL>" in v for v in r[:6]):
                nulls[why] += 1
            rows.append((why, r[6], tuple(strip_nulls(v) for v in r[:6])))
    print(f"{len(rows):,} US rows", file=sys.stderr)
    exp = [expected_of(r) for _, _, r in rows]

    tot = collections.defaultdict(collections.Counter)  # (style, group) -> counter
    by_state = collections.defaultdict(collections.Counter)
    by_source = collections.defaultdict(collections.Counter)
    clusters = collections.defaultdict(lambda: collections.defaultdict(list))
    dump = open(args.dump, "w") if args.dump else None

    for style in styles:
        print(f"style {style}", file=sys.stderr)
        if style == "fields":
            res = run_fields(args.addr, [r for _, _, r in rows], args.tmp)
        else:
            raw = run_lines(args.addr, [render(r, style) for _, _, r in rows])
            res = [(d["components"], d["warnings"], d.get("unparsed")) for d in raw]
        for (why, source, row), e0, (comp, warns, unparsed) in zip(rows, exp, res):
            e = dict(e0)  # canon() below must not change the shared expectation
            o = ours_of(comp)
            q = quirks(row)
            for k in CHECKS:
                if k == "street":
                    o[k], e[k] = canon(o[k]), canon(e[k])
            bad = [k for k in CHECKS if o[k] != e[k]]
            groups = ["all", why, f"{why}/{'quirk' if q else 'clean'}"]
            for g in groups:
                c = tot[(style, g)]
                c["n"] += 1
                for k in CHECKS:
                    c[k] += o[k] == e[k]
                c["whole"] += not bad
                c["unparsed"] += bool(unparsed)
                c["warned"] += bool(warns)
            if why == "uniform" and not q and style in ("commas", "fields"):
                st = by_state[(style, row[4])]
                st["n"] += 1
                st["whole"] += not bad
                sc = by_source[(style, source)]
                sc["n"] += 1
                sc["whole"] += not bad
            for k in bad:
                if style in ("commas", "fields") and not q:
                    clusters[(style, k)][first_diff(e[k], o[k]) if k in ("street", "number") else
                                         ("(none)" if not e[k] else "(value)", "(none)" if not o[k] else "(value)")
                                         ].append((row, e[k], o[k]))
                if dump:
                    dump.write(json.dumps({"style": style, "why": why, "source": source, "field": k, "quirk": q,
                                           "row": row, "expected": e[k], "ours": o[k],
                                           "warnings": warns, "unparsed": unparsed}) + "\n")

    md = ["# Netsyms comparison", "", f"{len(rows):,} US rows from the sample. Private data; do not commit.", ""]
    md += ["Rows with a literal `<NULL>` in a value (cleaned before scoring): "
           + ", ".join(f"{k} {v:,}" for k, v in sorted(nulls.items())) + ".", ""]
    md += ["## Accuracy by style and draw", "",
           "| style | draw | rows | whole | " + " | ".join(CHECKS) + " | unparsed | warned |",
           "|---|---|---|---|" + "---|" * (len(CHECKS) + 2)]
    for style in styles:
        for g in ["uniform/clean", "uniform/quirk", "uniform", "all", "unit/clean", "unit", "odd_number/clean",
                  "odd_number", "highway/clean", "highway", "quota/clean", "quota"]:
            c = tot.get((style, g))
            if c:
                md.append(f"| {style} | {g} | {c['n']:,} | {pct(c['whole'], c['n'])} | "
                          + " | ".join(pct(c[k], c["n"]) for k in CHECKS)
                          + f" | {pct(c['unparsed'], c['n'])} | {pct(c['warned'], c['n'])} |")
    md.append("")
    for style in ("commas", "fields"):
        if style not in styles:
            continue
        st = sorted(((k[1], c) for k, c in by_state.items() if k[0] == style and c["n"] >= 300),
                    key=lambda x: x[1]["whole"] / x[1]["n"])
        md += [f"## Lowest states, {style} style (uniform draw without source quirks, 300+ rows)", "",
               "| state | rows | whole |", "|---|---|---|"]
        md += [f"| {s} | {c['n']:,} | {pct(c['whole'], c['n'])} |" for s, c in st[:15]] + [""]
        sc = sorted(((k[1], c) for k, c in by_source.items() if k[0] == style and c["n"] >= 200),
                    key=lambda x: x[1]["whole"] / x[1]["n"])
        md += [f"## Lowest sources, {style} style (uniform draw without source quirks, 200+ rows)", "",
               "| source | rows | whole |", "|---|---|---|"]
        md += [f"| {s} | {c['n']:,} | {pct(c['whole'], c['n'])} |" for s, c in sc[:25]] + [""]
    for (style, field), cl in sorted(clusters.items()):
        total = sum(len(v) for v in cl.values())
        md += [f"## Mismatches: {field}, {style} style ({total:,})", "",
               "Rows that are not source quirks, grouped by the first differing tokens (expected, ours).", "",
               "| expected | ours | rows | example (expected / ours) |", "|---|---|---|---|"]
        for sig, items in sorted(cl.items(), key=lambda x: -len(x[1]))[:25]:
            row, e, o = items[0]
            md.append(f"| {sig[0]} | {sig[1]} | {len(items):,} | {e} / {o} |")
        md.append("")
    with open(args.out, "w") as f:
        f.write("\n".join(md) + "\n")
    print(f"wrote {args.out}", file=sys.stderr)


if __name__ == "__main__":
    main()
