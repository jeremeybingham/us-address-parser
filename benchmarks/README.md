# Benchmarks

Scripts behind `docs/benchmarks.md`. They measure accuracy, speed and size so
the numbers can be rerun. Each script prints Markdown to stdout (the machine,
the versions and the input are in the output) and changes nothing in the
repository. Private data stays in `local-data/` (`local-data/README.md`); the
scripts only print aggregate numbers from it, and skip a source that is not
there.

This library formats and parses; it does not validate deliverability, and none
of these numbers say an address exists.

## Setup

```sh
cargo build --release -p doorplate-cli          # target/release/doorplate
python -m venv .venv && . .venv/bin/activate
pip install maturin usaddress pandas polars
(cd crates/doorplate-py && maturin develop --release)
```

A debug build is much slower: always use `--release`.

## Scripts

| Script | Measures | Needs |
|---|---|---|
| `accuracy.py` | Whole-address and token accuracy: the golden corpus, usaddress's test files, the geocode benchmark (blind), the hand-labeled Florida sample | the CLI; `usaddress`; `local-data/` for the last two |
| `speed_python.py` | Addresses per second from Python: `parse`, `parse_many`, `parse_fields`, against `usaddress.parse` and `usaddress.tag` on the same inputs | the extension; `usaddress` |
| `speed_rust.py` | Rust speed: runs `cargo bench` (criterion) and a CLI file run, prints the medians | Rust; optional CSV |
| `size_memory.py` | Binary, wheel and installed size; import time; resident memory after import and after a batch; usaddress for comparison | the CLI; the extension; `usaddress`; `maturin` |

```sh
python benchmarks/accuracy.py
python benchmarks/speed_python.py [N]
python benchmarks/speed_rust.py [--csv FILE --column NAME]
python benchmarks/size_memory.py
```

Run on a quiet machine, with the power plan fixed. The speed scripts report the
best of several runs, which is the least noisy measure of a CPU-bound loop.
The existing `crates/doorplate-py/benchmarks/bench_batch.py` times pandas and
Polars frame building as well.
