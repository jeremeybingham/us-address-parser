"""Profile the Netsyms Address Database (private data; output stays local).

Counts and null rates come from one scan of the full file; shape tables
come from the sample written by sample.py (uniform rows only, so rates are
unbiased). No address is parsed. Writes markdown.

    python3 tools/netsyms/profile.py \
        --db local-data/netsyms-2026/2026-05-dbfull.sqlite \
        --sample local-data/netsyms-2026/sample.sqlite \
        --out local-data/netsyms-2026/profile.md
"""
import argparse
import collections
import re
import sqlite3


def pct(n, d):
    return f"{100 * (n or 0) / d:.1f}%" if d else "-"


def table(rows, header):
    out = ["| " + " | ".join(header) + " |", "|" + "---|" * len(header)]
    out += ["| " + " | ".join(str(x) for x in r) + " |" for r in rows]
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--db", required=True)
    ap.add_argument("--sample", required=True)
    ap.add_argument("--out", required=True)
    args = ap.parse_args()

    md = ["# Netsyms 2026-05 profile", ""]
    big = sqlite3.connect(f"file:{args.db}?mode=ro", uri=True)
    n_total = big.execute("select max(rowid) from addresses").fetchone()[0]
    md.append(f"Rows (max rowid): {n_total:,}. Private data; do not commit.\n")

    # One scan: everything that is a count.
    agg = big.execute(
        """
        select country, state, count(*),
               sum(street2 is not null and street2 <> ''),
               sum(number glob '*[^0-9]*'),
               sum(plus4 is not null and plus4 <> ''),
               sum(coalesce(plus4 like '%ND', 0)),
               sum(length(zipcode) <> 5 and country = 'US'),
               sum(street like '%HWY%' or street like '%HIGHWAY%' or street like '% ROUTE %'),
               sum(city = '' or city is null)
        from addresses group by country, state order by 3 desc
        """
    ).fetchall()
    rows = [
        (c, s, f"{n:,}", pct(u, n), pct(o, n), pct(p4, n), pct(nd, n), pct(bz, n), pct(hw, n), pct(nc, n))
        for c, s, n, u, o, p4, nd, bz, hw, nc in agg
    ]
    md.append("## By country and state (full file)\n")
    md += table(rows, ["country", "state", "rows", "unit", "non-digit number", "plus4", "plus4 ND",
                       "ZIP not 5", "highway-like", "no city"])
    md.append("")

    src = big.execute(
        "select source, count(*) from addresses group by source order by 2 desc"
    ).fetchall()
    md.append(f"## Sources ({len(src):,}; top 40 by rows)\n")
    md += table([(s or "(none)", f"{n:,}", pct(n, n_total)) for s, n in src[:40]], ["source", "rows", "share"])
    md.append("")

    # Shapes: the uniform draw of the sample.
    smp = sqlite3.connect(f"file:{args.sample}?mode=ro", uri=True)
    uni = smp.execute(
        "select number, street, street2, city, state, zipcode, plus4, country "
        "from sample where why = 'uniform' and country = 'US'"
    ).fetchall()
    n = len(uni)
    md.append(f"## Shapes (uniform US sample, {n:,} rows)\n")

    def top(counter, k=25):
        return table([(x, f"{v:,}", pct(v, n)) for x, v in counter.most_common(k)], ["shape", "rows", "share"])

    shape = collections.Counter(re.sub(r"[A-Za-z]", "a", re.sub(r"\d", "9", r[0])) for r in uni)
    md += ["### House number shapes", *top(shape), ""]
    last = collections.Counter((r[1].split() or [""])[-1] for r in uni)
    md += ["### Last word of street", *top(last, 40), ""]
    first = collections.Counter((r[1].split() or [""])[0] for r in uni)
    md += ["### First word of street", *top(first, 30), ""]
    words = collections.Counter(len(r[1].split()) for r in uni)
    md += ["### Words in street", *top(words, 12), ""]
    unit = collections.Counter(re.sub(r"[A-Za-z]", "a", re.sub(r"\d", "9", r[2])) for r in uni if r[2])
    md += ["### street2 shapes (rows with a unit)", *top(unit, 20), ""]
    chars = collections.Counter(ch for r in uni for ch in r[1] if not (ch.isalnum() or ch == " "))
    md += ["### Punctuation in street", *top(chars, 10), ""]
    lower = sum(1 for r in uni if r[1] != r[1].upper())
    md.append(f"Streets with lowercase letters: {pct(lower, n)}\n")
    nocity = sum(1 for r in uni if not r[3])
    md.append(f"Rows with no city: {pct(nocity, n)}; no ZIP: {pct(sum(1 for r in uni if not r[5]), n)}\n")
    with open(args.out, "w") as f:
        f.write("\n".join(md) + "\n")
    print(f"wrote {args.out}")


if __name__ == "__main__":
    main()
