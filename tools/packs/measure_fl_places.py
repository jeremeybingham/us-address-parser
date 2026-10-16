"""Report the cities in the DBPR licensee export that match no place.

    cargo build --release -p addr-cli
    uv run --with pyarrow python tools/packs/measure_fl_places.py \
        local-data/florida/dbpr_20251010_113032.parquet \
        local-data/florida/COSMETOLOGYLICENSE_1.csv ... [--top 60]

Each argument is a DBPR export (`.parquet` or board `.csv`; see `dbpr.py`).
Addresses are counted once across all of them.

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

sys.path.insert(0, str(Path(__file__).resolve().parent))
from dbpr import write_parse_input  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
ADDR = ROOT / "target" / "release" / "addr"


def main(paths, top):
    with tempfile.TemporaryDirectory() as tmp:
        src, out = Path(tmp) / "in.csv", Path(tmp) / "out.csv"
        nrows, distinct = write_parse_input(paths, src)
        subprocess.run([str(ADDR), "parse-file", str(src), "--column", "line1", "--line2", "line2",
                        "--city", "city", "--state", "state", "--zip", "zip", "-o", str(out)], check=True)
        keys, cnt = defaultdict(set), Counter()
        with out.open(encoding="utf-8") as f:
            for n, r in enumerate(csv.DictReader(f)):
                if r["addr_city"] and not r["addr_place"]:
                    z = r["addr_zip5"][:5]
                    keys[(r["addr_city"], z)].add(r["addr_key"] or f"row{n}")
                    cnt[(r["addr_city"], z)] += 1
    by_city = defaultdict(lambda: [0, 0, Counter()])
    for (c, z), ks in keys.items():
        by_city[c][0] += cnt[(c, z)]
        by_city[c][1] += len(ks)
        by_city[c][2][z] += len(ks)
    ranked = sorted(by_city.items(), key=lambda kv: -kv[1][1])
    print(f"{nrows} Florida rows ({distinct} distinct); {sum(v[0] for _, v in ranked)} rows "
          f"({sum(v[1] for _, v in ranked)} distinct addresses) in {len(ranked)} cities match no place")
    for c, (n, k, z) in ranked[:top]:
        print(f"{c}\t{n} rows\t{k} addresses\t" + ";".join(f"{a}:{b}" for a, b in z.most_common(4)))


if __name__ == "__main__":
    args = sys.argv[1:]
    top = 60
    if "--top" in args:
        i = args.index("--top")
        top = int(args[i + 1])
        del args[i : i + 2]
    if not args or not ADDR.exists():
        sys.exit("usage: measure_fl_places.py <dbpr export .parquet or .csv> ... [--top N] (build addr first)")
    main(args, top)
