# doorplate (Python)

Parse US addresses into components, with USPS Publication 28 and RESO
output, a match key, a URL-safe slug, and diagnostics. A Rust library with
Python bindings (PyO3, abi3 wheels for Python 3.10 and later); no runtime
dependencies and no data to download.

This library formats and parses; it **does not validate** that an address
exists or can receive mail.

```python
import doorplate as uap

a = uap.parse("123 north main street apt 4, Miami FL 33101")
a.street_name          # "MAIN"
a.unit_number          # "4"
a.place                # {"name": "MIAMI", "state": "FL", "county": "DADE", ...}
a.warnings             # []
a.to_usps()["line1"]   # "123 N MAIN ST APT 4"
a.key()                # "k1:S|123|N|MAIN|ST||FL|33101"
a.slug()               # "123-n-main-st-apt-4-miami-fl-33101"
a.to_reso()            # RESO Data Dictionary 2.0 fields
a.to_dict()            # everything above as plain Python values

uap.parse_fields(line1="THE COVE", line2="1111 ASBURY WAY",
                 city="Orlando", state="FL", zip="32801")
uap.parse_many(["1 Main St, Miami FL", "PO Box 5, Tampa FL"])  # GIL released
uap.parse("281 EVELYN DR DOTHAN", default_state="AL").city     # "DOTHAN"
```

- Unusable input (no address text at all) raises `ParseError`, a
  `ValueError`; in `parse_many` that element is `None`. Anything else comes
  back with `warnings`.
- `default_state` is a lookup hint for input with no state; it never fills
  `state`. A value that is not a US state, DC or territory raises
  `ValueError`.
- `confidence` is reserved and always `None` in this release (a calibrated
  score is deferred; `docs/design.md` decision 43).

## Install

Not published yet. Wheels are built for Linux, macOS and Windows on every pull
request; the package will be `doorplate` on PyPI (import
`doorplate`). Until then, build from source (below).

## Batches and pandas

`parse_many(texts)` takes a list or tuple of strings, releases the GIL and
parses in parallel with Rust threads, and returns the results in order (`None`
for an element with no address text). On 100,000 addresses on a 14-CPU
machine (`benchmarks/bench_batch.py`):

| | Time | Per second | Speed-up |
|---|---|---|---|
| a loop of `parse` | 1.55 s | 64,000 | 1x |
| `parse_many` | 0.25 s | 403,000 | 6.3x |
| `parse_many` and a pandas DataFrame of 7 columns | 0.45 s | 222,000 | 3.5x |
| `parse_many` and a Polars DataFrame of 7 columns | 0.34 s | 299,000 | 4.6x |

A `ParsedAddress` computes its formats when you ask, so take only what you
need. Neither pandas nor Polars is a dependency. The helpers below cover the
common case; the recipes after them are for when you want other fields.

**Helpers.** Install the extra (`pip install "doorplate[polars]"` or
`[pandas]`) and parse a column in one call:

```python
from doorplate.polars import parse_column   # or doorplate.pandas

out = df.hstack(parse_column(df["address"]))                    # Polars
out = df.join(parse_column(df["address"], default_state="FL"))  # pandas
```

`parse_column(series, *, columns=None, prefix="addr_", default_state=None)`
returns a DataFrame with one string column per name, in the same order (and,
in pandas, the same index) as the input. A null, blank or non-string row, or
one with no address text, is null in every column. The default columns are
`street_number`, `street_dir_prefix`, `street_name`, `street_suffix`,
`street_dir_suffix`, `unit_type`, `unit_number`, `city`, `state`, `zip5`,
`zip4`, `address_type`, `key`, `slug` and `warnings` (codes joined by commas).
`AVAILABLE_COLUMNS` in either module lists the rest (`key_unit`,
`usps_line1`, `usps_line2`, `usps_last_line`, `po_box`, `attention`,
`building`, `unparsed`, ...). They are built over `parse_many`; there is no
Polars expression namespace.

**Polars** (the better fit next to a Rust parser; building the frame was
taking about a quarter less time than pandas in the benchmark below):

```python
import polars as pl
import doorplate as uap

df = pl.read_csv("addresses.csv")
parsed = uap.parse_many(df["address"].fill_null("").to_list())  # None for a blank row

cols = ["street_number", "street_name", "street_suffix", "unit_number", "city", "state", "zip5"]
flat = pl.DataFrame(
    {f"addr_{c}": [getattr(a, c) if a else None for a in parsed] for c in cols}
    | {
        "addr_key": [a.key() if a else None for a in parsed],
        "addr_warnings": [",".join(a.warnings) if a else None for a in parsed],
    },
    schema={f"addr_{c}": pl.String for c in cols}
    | {"addr_key": pl.String, "addr_warnings": pl.String},
)
out = df.hstack(flat)
```

(Give the schema: a column that is `None` in every row would otherwise be
typed `Null`.) For every field, build structs from `to_dict()` and pick
fields from them:

```python
full = pl.DataFrame([a.to_dict() if a else None for a in parsed], infer_schema_length=None)
full.select(
    "street_name",
    pl.col("place").struct.field("name").alias("place_name"),
    pl.col("usps").struct.field("line1").alias("usps_line1"),
)
```

**pandas**:

```python
import pandas as pd
import doorplate as uap

df = pd.read_csv("addresses.csv")
parsed = uap.parse_many(df["address"].fillna("").tolist())  # None for a blank row

cols = ["street_number", "street_name", "street_suffix", "unit_number", "city", "state", "zip5"]
flat = pd.DataFrame([{c: getattr(a, c) for c in cols} if a else {} for a in parsed])
flat["key"] = [a.key() if a else None for a in parsed]
flat["warnings"] = [",".join(a.warnings) if a else None for a in parsed]
out = pd.concat([df, flat.add_prefix("addr_")], axis=1)
```

For every field with pandas, use `to_dict()` and
`pd.json_normalize([a.to_dict() if a else {} for a in parsed])`.

Calling `to_dict()` on every row costs about as much as parsing again (the
loop with `to_dict` ran at 55,000 per second), and reading attributes from
100,000 Python objects is now most of the time, so pick columns when you can.
Pre-split columns (`line1`, `city`, ...) have no batch call yet: loop over
`parse_fields`, or join the columns with commas and use `parse_many`.

## Coming from usaddress

| usaddress | `doorplate` |
|---|---|
| `usaddress.parse(s)` | `parse_tokens(s)`: `[(token, label)]` |
| `usaddress.tag(s)` | `tag(s)`: `(dict, "Street Address" \| "PO Box" \| "Ambiguous")` |
| `RepeatedLabelError` | not raised: the runs are joined and `parse(s).repeated_labels()` lists the labels |
| (none) | `parse(s).address_type()`: `street`, `po_box`, `rural_route`, `military`, `general_delivery`, `dual`, `unknown` |
| (none) | `parse(s, default_state="FL")`: a state hint for input with no state |

The differences (tokens that cannot be placed are left out of `tag()`, the
labels this library does not model) are in the repository README, "Coming from
usaddress".

## Wheels

Every pull request builds abi3 wheels (Linux x86_64 and aarch64, macOS
universal2, Windows x64) and tests each on Python 3.10 and 3.13
(`.github/workflows/wheels.yml`). Nothing is published yet.

## Build from source

```sh
pip install maturin
maturin develop      # in this directory; builds the extension into your venv
pytest tests
```
