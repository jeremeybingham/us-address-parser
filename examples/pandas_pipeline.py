"""Parse a CSV of addresses with pandas.

    pip install "doorplate[pandas]"
    python examples/pandas_pipeline.py [--out parsed.csv]

All addresses in examples/data are invented. This library formats and
parses; it does not validate that an address exists or can receive mail.
"""
import argparse
from pathlib import Path

import pandas as pd

import doorplate as uap
from doorplate.pandas import parse_column

DATA = Path(__file__).parent / "data"


def main(out: str | None = None) -> pd.DataFrame:
    pd.set_option("display.width", 200)
    pd.set_option("display.max_columns", None)
    pd.set_option("display.max_colwidth", 48)
    # 1. One-line addresses in a single column (a blank cell is read as NaN
    #    and comes back null).
    df = pd.read_csv(DATA / "addresses_oneline.csv")
    parsed = df.join(parse_column(df["address"]))
    print(parsed[["id", "addr_street_number", "addr_street_name", "addr_city", "addr_state", "addr_zip5"]])

    # 2. Keep what you need: pick columns, change the prefix, hint a state.
    small = parse_column(
        df["address"], columns=["key", "usps_line1", "usps_last_line"], prefix="", default_state="FL"
    )
    print(small.head(3))

    # 3. Find the rows that need a look: any warning, or text left over.
    check = parsed[parsed["addr_warnings"].fillna("") != ""][["id", "address", "addr_warnings"]]
    print(f"\n{len(check)} row(s) with warnings:")
    print(check)

    # 4. Duplicates by match key (rows with no address text have no key).
    counts = parsed["addr_key"].dropna().value_counts()
    print(f"{(counts > 1).sum()} duplicated key(s)")

    # 5. Pre-split columns: no batch helper yet, so loop over parse_fields.
    split = pd.read_csv(DATA / "addresses_split.csv", dtype=str)
    # pandas holds a missing cell as NaN; parse_fields wants None.
    def text(v):
        return v if isinstance(v, str) else None

    rows = [
        uap.parse_fields(
            line1=text(r["line1"]),
            line2=text(r["line2"]),
            city=text(r["city"]),
            state=text(r["state"]),
            zip=text(r["zip"]),
        )
        for r in split.to_dict("records")
    ]
    split["addr_key"] = [a.key() for a in rows]
    split["addr_slug"] = [a.slug() for a in rows]
    print(split)

    if out:
        parsed.to_csv(out, index=False)
        print(f"wrote {out}")
    return parsed


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", help="write the full parsed table to this CSV")
    main(ap.parse_args().out)
