# Examples

Runnable examples of the Python package and the `addr` CLI. Every address in
`data/` is invented. This library formats and parses; it does not validate
that an address exists or can receive mail.

| File | Shows |
|---|---|
| [`quickstart.ipynb`](quickstart.ipynb) | A tour: `parse`, the USPS and RESO forms, keys and slugs, messy lines and warnings, `default_state`, `parse_fields`, `parse_many`, a pandas frame, the CLI on a file |
| [`polars_pipeline.py`](polars_pipeline.py) | Polars: `parse_column` on a CSV, picking columns, finding rows with warnings, duplicates by key, pre-split columns |
| [`pandas_pipeline.py`](pandas_pipeline.py) | The same with pandas |
| [`data/addresses_oneline.csv`](data/addresses_oneline.csv) | One address per row in a single column (street, PO box, rural route, general delivery, a unit, a blank row) |
| [`data/addresses_split.csv`](data/addresses_split.csv) | Line 1, line 2, city, state and ZIP in separate columns, with gaps |
| [`data/addresses_messy.txt`](data/addresses_messy.txt) | One line per address as people type them: lowercase, `c/o`, a country, an intersection, no state |

## Run them

From the repository root, with the package built (`maturin develop` in
`crates/addr-py`) and the extras you need:

```sh
pip install pandas polars jupyter
python examples/polars_pipeline.py
python examples/pandas_pipeline.py --out /tmp/parsed.csv
jupyter lab examples/quickstart.ipynb
```

The CLI on a file (build it with `cargo build --release -p addr-cli`):

```sh
target/release/addr parse-file examples/data/addresses_oneline.csv --column address -o /tmp/parsed.csv
target/release/addr parse-file examples/data/addresses_split.csv --column line1 --line2 line2 --city city --state state --zip zip
target/release/addr parse - < examples/data/addresses_messy.txt
```

The tests in `crates/addr-py/tests/test_examples.py` run both scripts and the
notebook, so the examples stay current.
