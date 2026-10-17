"""Shared helpers for the benchmark scripts."""
import json
import os
import platform
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CLI = ROOT / "target" / "release" / "doorplate"
GEOCODE = ROOT / "local-data" / "geocode-benchmark"


def machine():
    """One line: CPU, cores, RAM, OS, Python, Rust."""
    cpu = "unknown CPU"
    try:
        for line in Path("/proc/cpuinfo").read_text().splitlines():
            if line.startswith("model name"):
                cpu = line.split(":", 1)[1].strip()
                break
    except OSError:
        cpu = platform.processor() or cpu
    ram = ""
    try:
        kb = int(re.search(r"MemTotal:\s+(\d+)", Path("/proc/meminfo").read_text()).group(1))
        ram = f", {kb / 1048576:.0f} GB RAM"
    except (OSError, AttributeError):
        pass
    try:
        rust = subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip()
    except OSError:
        rust = "rustc not found"
    return (f"{cpu}, {os.cpu_count()} threads{ram}; {platform.system()} {platform.release()}; "
            f"Python {platform.python_version()}; {rust}")


def corpus_inputs():
    """The single-line inputs of the golden corpus (committed, invented)."""
    out = []
    for path in sorted((ROOT / "tests" / "corpus").glob("*.jsonl")):
        for line in path.read_text().splitlines():
            if line.strip():
                s = json.loads(line).get("input")
                if s:
                    out.append(s)
    return out


def geocode_inputs():
    """The geocode benchmark's addresses as one-line strings, or [] if absent.
    The file is IOB-labeled: `TOKEN LABEL` lines, one blank line between
    addresses, `-END-` closing each."""
    path = GEOCODE / "test.txt"
    if not path.exists():
        return []
    out, cur = [], []
    for line in path.read_text().splitlines():
        parts = line.split()
        if not parts or parts[0] == "-END-":
            if cur:
                out.append(" ".join(cur))
                cur = []
            continue
        cur.append(parts[0])
    if cur:
        out.append(" ".join(cur))
    return out


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, check=True, **kw).stdout


def best(fn, runs=5):
    import time

    times = []
    for _ in range(runs):
        t = time.perf_counter()
        fn()
        times.append(time.perf_counter() - t)
    return min(times)


def need_cli():
    if not CLI.exists():
        sys.exit(f"{CLI} not found: cargo build --release -p doorplate-cli")
