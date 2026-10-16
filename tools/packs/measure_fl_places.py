"""Report the cities in the DBPR licensee export that match no place.

    cargo build --release -p addr-cli
    uv run --with pyarrow python tools/packs/measure_fl_places.py \
        local-data/florida/dbpr_20251010_113032.parquet [--top 60]

Parses the export's Florida addresses with `addr parse-file` and counts, for
each city with a city field and no place match, the rows and the distinct
street addresses (`k1` keys) that write it, with its most common ZIPs. This is
how the Florida pack's `places_added.tsv` and `variants_added.tsv` were
chosen (docs/packs/fl.md): a real community or a clear spelling of a known
place, at least 10 distinct addresses, one main ZIP. Typos and one-off
abbreviations stay out. Only city, ZIP and the street key leave the export;
licensee names never do.
"""
import csv, subprocess, sys, tempfile
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ADDR = ROOT / "target" / "release" / "addr"


def main(parquet, top):
    import pyarrow.parquet as pq

    cols = ["address_1", "address_2", "address_3", "city", "state_code", "zip_"]
    rows = [r for r in pq.read_table(parquet, columns=cols).to_pylist() if r["state_code"] == "FL"]
    with tempfile.TemporaryDirectory() as tmp:
        src, out = Path(tmp) / "in.csv", Path(tmp) / "out.csv"
        with src.open("w", newline="", encoding="utf-8") as f:
            w = csv.writer(f)
            w.writerow(["line1", "line2", "city", "state", "zip"])
            for r in rows:
                l2 = " ".join(x.strip() for x in (r["address_2"], r["address_3"]) if x and x.strip())
                w.writerow([r["address_1"] or "", l2, r["city"] or "", "FL", r["zip_"] or ""])
        subprocess.run([str(ADDR), "parse-file", str(src), "--column", "line1", "--line2", "line2",
                        "--city", "city", "--state", "state", "--zip", "zip", "-o", str(out)], check=True)
        keys, nrows = defaultdict(set), Counter()
        with out.open(encoding="utf-8") as f:
            for n, r in enumerate(csv.DictReader(f)):
                if r["addr_city"] and not r["addr_place"]:
                    z = r["addr_zip5"][:5]
                    keys[(r["addr_city"], z)].add(r["addr_key"] or f"row{n}")
                    nrows[(r["addr_city"], z)] += 1
    by_city = defaultdict(lambda: [0, 0, Counter()])
    for (c, z), ks in keys.items():
        by_city[c][0] += nrows[(c, z)]
        by_city[c][1] += len(ks)
        by_city[c][2][z] += len(ks)
    ranked = sorted(by_city.items(), key=lambda kv: -kv[1][1])
    print(f"{len(rows)} Florida rows; {sum(v[0] for _, v in ranked)} rows "
          f"({sum(v[1] for _, v in ranked)} distinct addresses) in {len(ranked)} cities match no place")
    for c, (n, k, z) in ranked[:top]:
        print(f"{c}\t{n} rows\t{k} addresses\t" + ";".join(f"{a}:{b}" for a, b in z.most_common(4)))


if __name__ == "__main__":
    args = sys.argv[1:]
    top = int(args[args.index("--top") + 1]) if "--top" in args else 60
    if not args or not ADDR.exists():
        sys.exit("usage: measure_fl_places.py <dbpr export .parquet> [--top N] (build addr first)")
    main(args[0], top)
