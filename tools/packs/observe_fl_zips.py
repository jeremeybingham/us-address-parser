"""Build the Florida pack's observed ZIP table (data/packs/fl/zips_observed.tsv)
from the DBPR real estate licensee export.

    cargo build --release -p addr-cli
    uv run --with pyarrow python tools/packs/observe_fl_zips.py \
        local-data/florida/dbpr_20251010_113032.parquet \
        local-data/florida/COSMETOLOGYLICENSE_1.csv ...

Each argument is a DBPR export (`.parquet` or board `.csv`; see `dbpr.py`).
Addresses are counted once across all of them.

GeoNames gives each ZIP one place, so a municipality that shares a ZIP with
its neighbour (CORAL GABLES in 33134, WESTON in 33331) raised
`zip_city_mismatch`. This script parses the export's Florida addresses with
`addr parse-file`, and lists each (ZIP, place) pair that `zips.tsv` does not
have, when enough distinct addresses use it:

- at least MIN_ADDRESSES distinct street addresses (`k1` keys) that are at
  least MIN_LARGE_SHARE of the ZIP's addresses, or
- at least MIN_SMALL addresses that are at least MIN_SHARE of the ZIP's
  addresses (small and PO Box ZIPs: MADEIRA BEACH in 33738).

A pair in KNOWN is kept with any count of addresses (at least one): places
whose ZIP is known from another source than the export's volume.

Only the city as a pack name or listed variant counts (not a cut-off city
or one taken from the ZIP), and only ZIPs `zips.tsv` already has. Pairs are
found against `zips.tsv`, not against the warning, so rerunning gives the
same table. Only city and ZIP leave the export; licensee names never do.
See docs/packs/fl.md.
"""
import csv, subprocess, sys, tempfile
from collections import defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from dbpr import write_parse_input  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
PACK = ROOT / "data" / "packs" / "fl"
ADDR = ROOT / "target" / "release" / "addr"
MIN_ADDRESSES = 10
MIN_SMALL = 3
MIN_SHARE = 0.05
# The MIN_ADDRESSES path also needs this share of the ZIP: with a million
# addresses, 10 is under 1% of a big ZIP and lets in a nearby city written
# for a suburb (ORLANDO in Winter Garden's 34787: 23 of 4,435).
MIN_LARGE_SHARE = 0.01
# (ZIP, place) pairs kept on fewer addresses, with why. BELLVIEW is a Census
# place (CDP, Escambia County) whose own ZIP is 32526, which GeoNames names
# PENSACOLA; 3 of the export's 363 addresses in the ZIP say BELLVIEW.
KNOWN = {("32526", "BELLVIEW")}


def fail(msg):
    sys.exit(f"observe_fl_zips: {msg}")


def main(paths):
    if not ADDR.exists():
        fail(f"{ADDR} not found; run cargo build --release -p addr-cli")
    base = {}
    with (PACK / "zips.tsv").open(encoding="utf-8") as f:
        for r in csv.DictReader(f, delimiter="\t"):
            base[r["zip"]] = {r["place"], *filter(None, r["shared"].split(";"))}
    with tempfile.TemporaryDirectory() as tmp:
        src, out = Path(tmp) / "in.csv", Path(tmp) / "out.csv"
        n, distinct = write_parse_input(paths, src)
        subprocess.run([str(ADDR), "parse-file", str(src), "--column", "line1",
                        "--line2", "line2", "--city", "city", "--state", "state",
                        "--zip", "zip", "-o", str(out)], check=True)
        per_zip, per_pair = defaultdict(set), defaultdict(set)
        with out.open(encoding="utf-8") as f:
            for r in csv.DictReader(f):
                z, place, key = r["addr_zip5"], r["addr_place"], r["addr_key"]
                if z not in base or not key or r["addr_place_source"] not in ("name", "variant"):
                    continue
                per_zip[z].add(key)
                if place not in base[z]:
                    per_pair[(z, place)].add(key)
    rows = []
    for (z, place), keys in per_pair.items():
        c = len(keys)
        share = c / len(per_zip[z])
        if (z, place) in KNOWN or (c >= MIN_ADDRESSES and share >= MIN_LARGE_SHARE) or (
            c >= MIN_SMALL and share >= MIN_SHARE
        ):
            rows.append((z, place, c))
    rows.sort()
    with (PACK / "zips_observed.tsv").open("w", encoding="utf-8", newline="") as f:
        f.write("zip\tplace\taddresses\n")
        for z, place, c in rows:
            f.write(f"{z}\t{place}\t{c}\n")
    print(f"observe_fl_zips: {n} Florida rows ({distinct} distinct), {len(per_pair)} pairs not in zips.tsv, "
          f"{len(rows)} kept -> {PACK / 'zips_observed.tsv'}")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        fail("usage: observe_fl_zips.py <dbpr export .parquet or .csv> ...")
    main(sys.argv[1:])
