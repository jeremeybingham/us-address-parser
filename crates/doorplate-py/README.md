# doorplate

[![CI](https://github.com/jeremeybingham/doorplate/actions/workflows/ci.yml/badge.svg)](https://github.com/jeremeybingham/doorplate/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/jeremeybingham/doorplate#license)

Parse US addresses into components, with USPS Publication 28 and RESO output, a
match key, a URL-safe slug, and diagnostics. A Rust library with Python bindings
(PyO3, abi3 wheels for Python 3.10 and later); no runtime dependencies and no data
to download.

This library formats and parses; it **does not validate** that an address exists or
can receive mail.

> **Alpha release (0.1.0).** Expect rough edges. The match key is versioned (`k1:`) and
> will probably get new versions as the parser improves, so store the version prefix
> with your keys and plan to re-key. Accuracy and speed are measurements on a few
> datasets, not guarantees.
>
> **Help wanted: messy addresses.** If it parses your addresses badly, please
> [open an issue](https://github.com/jeremeybingham/doorplate/issues/new?template=misparse-or-messy-data.md)
> with a sample. I want to fix them. Private data can be anonymized first
> ([how](https://github.com/jeremeybingham/doorplate/blob/main/CONTRIBUTING.md#sharing-messy-data)).

## Quickstart

```sh
pip install doorplate
```

```python
import doorplate as dp

a = dp.parse("123 north main street apt 4, Miami FL 33101")
a.street_name          # "MAIN"
a.unit_number          # "4"
a.place                # {"name": "MIAMI", "state": "FL", "county": "DADE", ...}
a.warnings             # []
a.to_usps()["line1"]   # "123 N MAIN ST APT 4"
a.key()                # "k1:S|123|N|MAIN|ST||FL|33101"
a.slug()               # "123-n-main-st-apt-4-miami-fl-33101"
a.to_reso()            # RESO Data Dictionary 2.0 fields
a.to_dict()            # everything above as plain Python values

dp.parse_fields(line1="THE COVE", line2="1111 ASBURY WAY",
                city="Orlando", state="FL", zip="32801")
dp.parse_many(["1 Main St, Miami FL", "PO Box 5, Tampa FL"])  # GIL released
dp.parse("281 EVELYN DR DOTHAN", default_state="AL").city     # "DOTHAN"
```

- Unusable input (no address text at all) raises `ParseError`, a `ValueError`; in
  `parse_many` that element is `None`. Anything else comes back with `warnings`.
- `default_state` is a lookup hint for input with no state; it never fills `state`.
  A value that is not a US state, DC or territory raises `ValueError`.
- `confidence` is reserved and always `None`: no score is computed.

## Install

```sh
pip install doorplate                  # the parser
pip install "doorplate[pandas]"        # plus the pandas helper
pip install "doorplate[polars]"        # plus the Polars helper
```

Python 3.10 or later. Wheels are built for Linux (x86_64, aarch64), macOS
(universal2) and Windows (x64), and tested on Python 3.10 and 3.13. To build from
source (Rust 1.83 or later):

```sh
pip install maturin
cd crates/doorplate-py
maturin develop --release      # builds the extension into your virtualenv
pytest tests
```

## The API

| Call | Returns |
|---|---|
| `parse(text, default_state=None)` | a `ParsedAddress` |
| `parse_fields(line1=, line2=, city=, state=, zip=, default_state=None)` | a `ParsedAddress` from pre-split fields |
| `parse_many(texts, default_state=None)` | a list of `ParsedAddress` (or `None` for an element with no address text), in order; parses in parallel with the GIL released |
| `Parser(default_state=None)` | an object with the same `parse`, `parse_fields` and `parse_many` |
| `tag(text)` | `(dict, type)` with usaddress's labels and `"Street Address"` / `"PO Box"` / `"Ambiguous"` |
| `parse_tokens(text)` | `[(token, label)]` with usaddress's labels |
| `address_type(text)` | `street`, `po_box`, `rural_route`, `military`, `general_delivery`, `dual` or `unknown` |

A `ParsedAddress` is read-only and typed (`py.typed`, stubs in `__init__.pyi`). It
has the components (`street_number`, `street_dir_prefix`, `street_name`,
`street_suffix`, `street_dir_suffix`, `unit_type`, `unit_number`,
`second_unit_type`, `second_unit_number`, `po_box`, `attention`, `building`, `city`,
`state`, `zip5`, `zip4`, `military_post_office`), the structured shapes (`highway`,
`rural_route`, `military`, `place`, plain dicts), `general_delivery`, `warnings`
(stable snake_case codes), `unparsed`, `input` and `confidence`, and the methods
`key(unit=False)`, `slug()`, `to_usps(max_line=None)`, `to_reso()`,
`to_usaddress_tags()`, `tag()`, `address_type()`, `repeated_labels()` and
`to_dict()`.

A `ParsedAddress` computes its output formats when you ask, so take only what you
need. The key is a versioned contract
([key format](https://github.com/jeremeybingham/doorplate/blob/main/docs/key-format.md));
the slug is for display.

## Batches, pandas and Polars

`parse_many(texts)` takes a list or tuple of strings, releases the GIL, parses in
parallel with Rust threads, and returns the results in order. On 100,000 addresses
on a 14-thread machine (`crates/doorplate-py/benchmarks/bench_batch.py`):

| | Time | Per second | Speed-up |
|---|---|---|---|
| a loop of `parse` | 2.11 s | 47,000 | 1x |
| `parse_many` | 0.32 s | 315,000 | 6.6x |
| `parse_many` and a pandas DataFrame of 7 columns | 0.49 s | 205,000 | 4.3x |
| `parse_many` and a Polars DataFrame of 7 columns | 0.40 s | 252,000 | 5.3x |

**Helpers.** Install the extra and parse a column in one call:

```python
from doorplate.polars import parse_column   # or doorplate.pandas

out = df.hstack(parse_column(df["address"]))                    # Polars
out = df.join(parse_column(df["address"], default_state="FL"))  # pandas
```

`parse_column(series, *, columns=None, prefix="addr_", default_state=None)` returns a
DataFrame with one string column per name, in the same order (and, in pandas, the
same index) as the input. A null, blank or non-string row, or one with no address
text, is null in every column. The default columns are `street_number`,
`street_dir_prefix`, `street_name`, `street_suffix`, `street_dir_suffix`,
`unit_type`, `unit_number`, `city`, `state`, `zip5`, `zip4`, `address_type`, `key`,
`slug` and `warnings` (codes joined by commas). `AVAILABLE_COLUMNS` in either module
lists the rest (`key_unit`, `usps_line1`, `usps_line2`, `usps_last_line`, `po_box`,
`attention`, `building`, `unparsed`, ...). They are built over `parse_many`.

**Polars by hand** (when you want other fields):

```python
import polars as pl
import doorplate as dp

df = pl.read_csv("addresses.csv")
parsed = dp.parse_many(df["address"].fill_null("").to_list())  # None for a blank row

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

(Give the schema: a column that is `None` in every row would otherwise be typed
`Null`.) For every field, build structs from `to_dict()` and pick fields from them:

```python
full = pl.DataFrame([a.to_dict() if a else None for a in parsed], infer_schema_length=None)
full.select(
    "street_name",
    pl.col("place").struct.field("name").alias("place_name"),
    pl.col("usps").struct.field("line1").alias("usps_line1"),
)
```

**pandas by hand:**

```python
import pandas as pd
import doorplate as dp

df = pd.read_csv("addresses.csv")
parsed = dp.parse_many(df["address"].fillna("").tolist())  # None for a blank row

cols = ["street_number", "street_name", "street_suffix", "unit_number", "city", "state", "zip5"]
flat = pd.DataFrame([{c: getattr(a, c) for c in cols} if a else {} for a in parsed])
flat["key"] = [a.key() if a else None for a in parsed]
flat["warnings"] = [",".join(a.warnings) if a else None for a in parsed]
out = pd.concat([df, flat.add_prefix("addr_")], axis=1)
```

For every field with pandas, use `to_dict()` and
`pd.json_normalize([a.to_dict() if a else {} for a in parsed])`.

Calling `to_dict()` on every row costs about as much as parsing again (a loop with
`to_dict` ran at 24,000 per second), and reading attributes from 100,000 Python
objects is now most of the time, so pick columns when you can. Pre-split columns
(`line1`, `city`, ...) have no batch call: loop over `parse_fields`, or join the
columns with commas and use `parse_many`.

## Coming from usaddress

| usaddress | `doorplate` |
|---|---|
| `usaddress.parse(s)` | `parse_tokens(s)`: `[(token, label)]` |
| `usaddress.tag(s)` | `tag(s)`: `(dict, "Street Address" \| "PO Box" \| "Ambiguous")` |
| `RepeatedLabelError` | not raised: the runs are joined and `parse(s).repeated_labels()` lists the labels |
| (none) | `parse(s).address_type()`: `street`, `po_box`, `rural_route`, `military`, `general_delivery`, `dual`, `unknown` |
| (none) | `parse(s, default_state="FL")`: a state hint for input with no state |

Tokens that cannot be placed are left out of `tag()`, and some usaddress labels
(landmarks, corners, recipients) are not modeled. The mapping and the differences:
[usaddress compatibility](https://github.com/jeremeybingham/doorplate/blob/main/docs/usaddress-compat.md).

## More

- [Project README](https://github.com/jeremeybingham/doorplate#readme): what it does,
  the Rust library, the command-line tool.
- [How it works](https://github.com/jeremeybingham/doorplate/blob/main/docs/how-it-works.md),
  [benchmarks](https://github.com/jeremeybingham/doorplate/blob/main/docs/benchmarks.md)
  and the [roadmap](https://github.com/jeremeybingham/doorplate/blob/main/docs/roadmap.md)
  (a confidence score and customization packs are planned).
- [Examples](https://github.com/jeremeybingham/doorplate/tree/main/examples): a
  notebook tour and DataFrame pipelines.

Licensed under either of MIT or Apache-2.0 at your option.
