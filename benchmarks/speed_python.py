#!/usr/bin/env python3
"""Python speed: doorplate against usaddress on the same strings.

    python benchmarks/speed_python.py [N]

Build the extension with `maturin develop --release` first (a debug build is
much slower). The inputs are the golden corpus's one-line addresses (committed,
invented) and, when `local-data/` has it, the geocode benchmark's addresses
(30,622 one-line strings; only timings are printed). Each is repeated up to N
(default 100,000). Every figure is the best of five runs. Prints Markdown.

usaddress is a CRF tagger (python-crfsuite); one call is `usaddress.parse` (the
token labels) or `usaddress.tag` (the joined dict). doorplate's `parse` also
does what `tag` does and more (components, key, slug are computed on demand), so
the closest like-for-like rows are `parse` / `tag` and `parse_tokens` /
`usaddress.parse`.
"""
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from _common import best, corpus_inputs, geocode_inputs, machine  # noqa: E402

import doorplate as dp  # noqa: E402

try:
    import usaddress
except ImportError:
    usaddress = None


def rate(n, seconds):
    return f"{n / seconds:,.0f}"


def table(name, texts):
    n = len(texts)
    rows = [
        ("doorplate `parse`", lambda: [dp.parse(t) for t in texts]),
        ("doorplate `parse` and `key`, `to_usps`", lambda: [(a.key(), a.to_usps()) for a in map(dp.parse, texts)]),
        ("doorplate `tag`", lambda: [dp.tag(t) for t in texts]),
        ("doorplate `parse_tokens`", lambda: [dp.parse_tokens(t) for t in texts]),
        ("doorplate `parse_many`", lambda: dp.parse_many(texts)),
    ]
    if usaddress is not None:
        rows += [
            ("usaddress `parse`", lambda: [usaddress.parse(t) for t in texts]),
            ("usaddress `tag`", lambda: [_tag(t) for t in texts]),
        ]
    results = []
    for label, fn in rows:
        results.append((label, best(fn)))
    base = dict(results)
    print(f"### {name} ({n:,} addresses)\n")
    print("| | Time | Addresses per second | Microseconds each |\n|---|---|---|---|")
    for label, secs in results:
        print(f"| {label} | {secs:.3f} s | {rate(n, secs)} | {secs / n * 1e6:,.1f} |")
    if usaddress is not None:
        ours = base["doorplate `parse_tokens`"]
        theirs = base["usaddress `parse`"]
        print(f"\nOne thread, call by call: `parse_tokens` is {theirs / ours:.1f}x `usaddress.parse`. "
              f"`parse_many` parses on all {os.cpu_count()} threads with the GIL released, "
              f"{theirs / base['doorplate `parse_many`']:.1f}x `usaddress.parse` (which is single-threaded).\n")
    else:
        print()


def _tag(t):
    try:
        return usaddress.tag(t)
    except usaddress.RepeatedLabelError:
        return None


def main():
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 100_000
    print(f"# Python speed\n\nMachine: {machine()}")
    print(f"doorplate {dp.__version__}" + (f"; usaddress {_usaddress_version()}" if usaddress else "; usaddress not installed") + "\n")
    base = corpus_inputs()
    table("Golden corpus inputs", (base * (n // len(base) + 1))[:n])
    geo = geocode_inputs()
    if geo:
        table("Geocode benchmark addresses", (geo * (n // len(geo) + 1))[:n])


def _usaddress_version():
    from importlib.metadata import version

    return version("usaddress")


if __name__ == "__main__":
    main()
