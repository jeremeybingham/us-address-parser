"""Time `parse_many` against a Python loop of `parse`.

    maturin develop --release        # in crates/addr-py
    python crates/addr-py/benchmarks/bench_batch.py [N]

The input is the golden corpus's single-line addresses (tests/corpus),
repeated to N (default 100,000). Each timing is the best of three runs. With
`to_dict`, the Python objects are turned into dicts too. The last rows build
a pandas and a polars DataFrame of seven chosen columns from the parsed
addresses (the README recipes), when those libraries are installed.
"""
import json
import os
import pathlib
import sys
import time

import us_address_parser as uap

ROOT = pathlib.Path(__file__).resolve().parents[3]


def corpus_inputs():
    out = []
    for path in sorted((ROOT / "tests" / "corpus").glob("*.jsonl")):
        for line in path.read_text().splitlines():
            if line.strip():
                s = json.loads(line).get("input")
                if s:
                    out.append(s)
    return out


def best(fn, runs=3):
    times = []
    for _ in range(runs):
        t = time.perf_counter()
        fn()
        times.append(time.perf_counter() - t)
    return min(times)


def main():
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 100_000
    base = corpus_inputs()
    texts = (base * (n // len(base) + 1))[:n]
    print(f"{n} addresses ({len(base)} distinct), {os.cpu_count()} CPUs, "
          f"library {uap.__version__}")
    rows = [
        ("loop of parse", lambda: [uap.parse(s) for s in texts]),
        ("parse_many", lambda: uap.parse_many(texts)),
        ("loop of parse + to_dict", lambda: [uap.parse(s).to_dict() for s in texts]),
        ("parse_many + to_dict", lambda: [a.to_dict() for a in uap.parse_many(texts) if a]),
    ]
    cols = ["street_number", "street_name", "street_suffix", "unit_number",
            "city", "state", "zip5"]
    try:
        import pandas as pd

        rows.append((
            "parse_many -> pandas",
            lambda: pd.DataFrame(
                [{c: getattr(a, c) for c in cols} if a else {}
                 for a in uap.parse_many(texts)]
            ),
        ))
    except ImportError:
        pass
    try:
        import polars as pl

        def to_polars():
            parsed = uap.parse_many(texts)
            return pl.DataFrame(
                {c: [getattr(a, c) if a else None for a in parsed] for c in cols},
                schema={c: pl.String for c in cols},
            )

        rows.append(("parse_many -> polars", to_polars))
    except ImportError:
        pass
    base_t = None
    for name, fn in rows:
        t = best(fn)
        base_t = base_t or t
        print(f"{name:26} {t:7.3f} s  {n / t:10,.0f} addresses/s  {base_t / t:5.1f}x")


if __name__ == "__main__":
    main()
