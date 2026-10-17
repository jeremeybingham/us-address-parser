"""Parse a CSV of addresses with Polars.

    pip install "doorplate[polars]"
    python examples/polars_pipeline.py [--out parsed.csv]

All addresses in examples/data are invented. This library formats and
parses; it does not validate that an address exists or can receive mail.
"""
import argparse
from pathlib import Path

import polars as pl

import doorplate as dp
from doorplate.polars import parse_column

DATA = Path(__file__).parent / "data"


def main(out: str | None = None) -> pl.DataFrame:
    # 1. One-line addresses in a single column.
    df = pl.read_csv(DATA / "addresses_oneline.csv")
    parsed = df.hstack(parse_column(df["address"]))
    print(parsed.select("id", "addr_street_number", "addr_street_name", "addr_city", "addr_state", "addr_zip5"))

    # 2. Keep what you need: pick columns, change the prefix, hint a state.
    small = parse_column(
        df["address"], columns=["key", "usps_line1", "usps_last_line"], prefix="", default_state="FL"
    )
    print(small.head(3))

    # 3. Find the rows that need a look: any warning, or text left over.
    check = parsed.filter(pl.col("addr_warnings").fill_null("") != "").select("id", "address", "addr_warnings")
    print(f"\n{check.height} row(s) with warnings:")
    print(check)

    # 4. Duplicates by match key (rows with no address text have no key).
    dupes = (
        parsed.filter(pl.col("addr_key").is_not_null())
        .group_by("addr_key")
        .len()
        .filter(pl.col("len") > 1)
    )
    print(f"{dupes.height} duplicated key(s)")

    # 5. Pre-split columns: no batch helper yet, so loop over parse_fields.
    split = pl.read_csv(DATA / "addresses_split.csv", infer_schema_length=0)
    rows = [
        dp.parse_fields(
            line1=r["line1"], line2=r["line2"], city=r["city"], state=r["state"], zip=r["zip"]
        )
        for r in split.iter_rows(named=True)
    ]
    print(split.hstack(pl.DataFrame({"addr_key": [a.key() for a in rows], "addr_slug": [a.slug() for a in rows]})))

    if out:
        parsed.write_csv(out)
        print(f"wrote {out}")
    return parsed


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", help="write the full parsed table to this CSV")
    main(ap.parse_args().out)
