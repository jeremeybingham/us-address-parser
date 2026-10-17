#!/usr/bin/env python3
"""Rust speed: the criterion benchmarks and a CLI file run.

    python benchmarks/speed_rust.py [--csv FILE --column NAME [--line2 C --city C --state C --zip C]]

Runs `cargo bench -p doorplate --bench parse --features rayon` (benches/parse.rs:
single addresses by shape, the golden corpus one by one and with `parse_many`,
and the emitters) and prints the median of each. With --csv it also times
`doorplate parse-file` on that file (one thread, CSV in and out, so it includes
reading, writing and about 30 output columns). Prints Markdown.
"""
import argparse
import re
import subprocess
import sys
import tempfile
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from _common import CLI, ROOT, machine, need_cli  # noqa: E402

UNIT = {"ns": 1e-9, "µs": 1e-6, "us": 1e-6, "ms": 1e-3, "s": 1.0}


def seconds(value, unit):
    return float(value) * UNIT[unit]


def fmt(sec):
    return f"{sec * 1e6:,.2f} µs" if sec < 1e-3 else f"{sec * 1e3:,.2f} ms"


def criterion():
    cmd = ["cargo", "bench", "-p", "doorplate", "--bench", "parse", "--features", "rayon",
           "--", "--warm-up-time", "1", "--measurement-time", "3"]
    out = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, check=True)
    text = out.stdout + out.stderr
    rows, name = [], None
    for line in text.splitlines():
        m = re.match(r"^(\S+/\S+)\s*(?:time:\s*\[(.*)\])?\s*$", line)
        if m:
            name = m.group(1)
            if not m.group(2):
                continue
            line = f"time: [{m.group(2)}]"
        t = re.search(r"time:\s+\[([\d.]+) (\S+) ([\d.]+) (\S+) ([\d.]+) (\S+)\]", line)
        if t and name:
            rows.append((name, seconds(t.group(3), t.group(4))))
            name = None
    return rows


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--csv")
    ap.add_argument("--column", default="address")
    for o in ("line2", "city", "state", "zip"):
        ap.add_argument(f"--{o}")
    a = ap.parse_args()
    print(f"# Rust speed\n\nMachine: {machine()}\n")
    rows = criterion()
    if not rows:
        sys.exit("no criterion output parsed")
    corpus_n = None
    print("## Criterion (median)\n\n| Benchmark | Time |\n|---|---|")
    for name, sec in rows:
        print(f"| `{name}` | {fmt(sec)} |")
    print()
    if a.csv:
        need_cli()
        cmd = [str(CLI), "parse-file", a.csv, "--column", a.column]
        for o in ("line2", "city", "state", "zip"):
            if getattr(a, o):
                cmd += [f"--{o}", getattr(a, o)]
        with tempfile.TemporaryDirectory() as d:
            out = str(Path(d) / "out.csv")
            n = sum(1 for _ in open(a.csv, "rb")) - 1
            best = None
            for _ in range(3):
                t = time.perf_counter()
                subprocess.run(cmd + ["-o", out], capture_output=True, check=True)
                dt = time.perf_counter() - t
                best = dt if best is None else min(best, dt)
        print(f"## CLI `parse-file`\n\n{n:,} rows in {best:.2f} s ({n / best:,.0f} rows per second, best of 3).\n")


if __name__ == "__main__":
    main()
