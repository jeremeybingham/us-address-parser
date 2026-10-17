"""Draw a working sample from the full Netsyms Address Database.

The full file (2026-05, about 175M rows, 40 GB) has no index on state or
source, so every filter scans it. This script scans it a few times and
writes a small SQLite file that all later work reads. The data is private
(docs/future-sources.md): both files stay in local-data/, never committed.

    python3 tools/netsyms/sample.py \
        --db local-data/netsyms-2026/2026-05-dbfull.sqlite \
        --out local-data/netsyms-2026/sample.sqlite

The sample is the union of three draws, each by a hash of the rowid, so a
rerun gives the same rows:
  quota    about --quota rows from every (state, source), so small sources
           and small states are present (rate = min(1, quota / count));
  uniform  1 in --uniform rows overall, for unbiased rates;
  shapes   rare shapes, at their own rates: a unit in street2, a house
           number that is not plain digits, a highway-looking street.
Each row records why it was drawn in `why` (the first draw that took it).
"""
import argparse
import sqlite3
import sys
import time

HASH = "((a.rowid * 2654435761) % 1000003)"  # 0 .. 1000002, near uniform
M = 1000003


def log(msg):
    print(f"{time.strftime('%H:%M:%S')} {msg}", file=sys.stderr, flush=True)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--db", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--quota", type=int, default=300)
    ap.add_argument("--uniform", type=int, default=500, help="1 in N rows")
    ap.add_argument("--unit-rate", type=int, default=100, help="1 in N rows with a unit")
    ap.add_argument("--odd-number-rate", type=int, default=5, help="1 in N odd house numbers")
    ap.add_argument("--highway-rate", type=int, default=100, help="1 in N highway-like streets")
    args = ap.parse_args()

    c = sqlite3.connect(args.out)
    c.execute("pragma journal_mode=off")
    c.execute("pragma synchronous=off")
    c.execute(f"attach database 'file:{args.db}?mode=ro' as big")  # noqa: S608
    c.executescript(
        """
        drop table if exists sample;
        drop table if exists strata;
        create table sample (
            src_rowid integer primary key,
            zipcode text, number text, street text, street2 text, city text,
            state text, plus4 text, country text, latitude real, longitude real,
            source text, why text);
        """
    )
    cols = "zipcode, number, street, street2, city, state, plus4, country, latitude, longitude, source"
    acols = ", ".join(f"a.{k.strip()}" for k in cols.split(","))

    log("counting strata (one full scan)")
    c.execute(
        "create table strata as select country, state, source, count(*) as n "
        "from big.addresses group by country, state, source"
    )
    c.execute("create index strata_k on strata(country, state, source)")
    total = c.execute("select sum(n) from strata").fetchone()[0]
    log(f"{total:,} rows, {c.execute('select count(*) from strata').fetchone()[0]:,} strata")

    def draw(why, where, rate_sql=None):
        """INSERT OR IGNORE the rows that pass `where`; first draw wins."""
        t = time.time()
        if rate_sql:
            sql = (
                f"insert or ignore into sample select a.rowid, {acols}, '{why}' "
                f"from big.addresses a join strata s on s.country = a.country "
                f"and s.state = a.state and s.source is a.source "
                f"where {where} and {HASH} < ({rate_sql}) * {M}"
            )
        else:
            sql = (
                f"insert or ignore into sample select a.rowid, {acols}, '{why}' "
                f"from big.addresses a where {where}"
            )
        before = c.execute("select count(*) from sample").fetchone()[0]
        c.execute(sql)
        c.commit()
        after = c.execute("select count(*) from sample").fetchone()[0]
        log(f"{why}: +{after - before:,} rows ({time.time() - t:.0f}s)")

    draw("quota", "1=1", f"min(1.0, {args.quota}.0 / s.n)")
    draw("uniform", f"{HASH} < {M // args.uniform}")
    draw("unit", f"a.street2 is not null and a.street2 <> '' and {HASH} < {M // args.unit_rate}")
    draw("odd_number", f"a.number glob '*[^0-9]*' and {HASH} < {M // args.odd_number_rate}")
    draw(
        "highway",
        "(a.street like '%HWY%' or a.street like '%HIGHWAY%' or a.street like '% ROUTE %' "
        "or a.street like 'SR %' or a.street like 'CR %' or a.street like 'FM %' "
        "or a.street like 'US %' or a.street like '% BYP%') "
        f"and {HASH} < {M // args.highway_rate}",
    )
    c.execute("create index sample_k on sample(country, state, source)")
    c.commit()
    n = c.execute("select count(*) from sample").fetchone()[0]
    log(f"sample: {n:,} rows")
    for why, k in c.execute("select why, count(*) from sample group by why order by 2 desc"):
        log(f"  {why}: {k:,}")
    c.execute("vacuum")


if __name__ == "__main__":
    main()
