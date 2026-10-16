"""Read the Florida addresses of Florida DBPR licensee exports (public
record) for the pack scripts (`observe_fl_zips.py`, `measure_fl_places.py`).

Two layouts, chosen by file extension:

- `.parquet`: the full real estate export, columns `address_1` to
  `address_3`, `city`, `state_code`, `zip_`;
- `.csv`: the board files (`RE_rgn4.csv`, `COSMETOLOGYLICENSE_1.csv`,
  `CONSTRUCTIONLICENSE_1.csv`, `lic03bb.csv`), no header, address in columns
  5 to 10 (line 1, line 2, line 3, city, state, ZIP).

Only the address fields are read; licensee names never leave the file.
"""
import csv
from pathlib import Path


def florida_rows(path):
    """Yield (line1, line2, city, zip) for each Florida row of `path`."""
    path = Path(path)
    if path.suffix == ".parquet":
        import pyarrow.parquet as pq

        cols = ["address_1", "address_2", "address_3", "city", "state_code", "zip_"]
        for r in pq.read_table(path, columns=cols).to_pylist():
            if r["state_code"] == "FL":
                l2 = " ".join(x.strip() for x in (r["address_2"], r["address_3"]) if x and x.strip())
                yield (r["address_1"] or "", l2, r["city"] or "", r["zip_"] or "")
    elif path.suffix == ".csv":
        with path.open(encoding="latin-1", newline="") as f:
            for r in csv.reader(f):
                if len(r) < 11:
                    continue
                l1, l2, l3, city, state, zip_ = (x.strip() for x in r[5:11])
                if state == "FL":
                    yield (l1, " ".join(x for x in (l2, l3) if x), city, zip_)
    else:
        raise SystemExit(f"dbpr: {path}: expected a .parquet or .csv export")


def write_parse_input(paths, out):
    """Write the distinct Florida addresses of all `paths` as one CSV for
    `addr parse-file` (columns line1, line2, city, state, zip). Returns
    (rows read, distinct rows written)."""
    seen = set()
    n = 0
    with Path(out).open("w", newline="", encoding="utf-8") as f:
        w = csv.writer(f)
        w.writerow(["line1", "line2", "city", "state", "zip"])
        for p in paths:
            for l1, l2, city, z in florida_rows(p):
                n += 1
                k = (l1, l2, city, z)
                if k not in seen:
                    seen.add(k)
                    w.writerow([l1, l2, city, "FL", z])
    return n, len(seen)
