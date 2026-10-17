#!/usr/bin/env python3
"""Accuracy: golden corpus, usaddress's test files, geocode benchmark, Florida sample.

    python benchmarks/accuracy.py

Needs the release CLI and `usaddress` (pip install usaddress). The geocode
benchmark and the Florida sample are in `local-data/` (private); a missing one
is skipped. Prints Markdown.
"""
import re
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from _common import CLI, FLORIDA_SAMPLE, GEOCODE, ROOT, machine, need_cli, run  # noqa: E402


def corpus_report(directory):
    """`whole address: 187/190 (98.4%)` and the case count from `corpus test`."""
    out = run([str(CLI), "corpus", "test", "--dir", str(directory)])
    cases = int(re.search(r"golden corpus: (\d+) cases", out).group(1))
    m = re.search(r"whole address:\s+(\d+)/(\d+) \(([\d.]+)%\)", out)
    return cases, int(m.group(1)), int(m.group(2)), m.group(3)


def compare(mode, *args):
    """Run tools/compare/compare_usaddress.py and return the report text."""
    with tempfile.TemporaryDirectory() as d:
        out = Path(d) / "report.md"
        run([sys.executable, str(ROOT / "tools/compare/compare_usaddress.py"), "--out", str(out), mode, *args])
        return out.read_text()


def table_row(report, first_cell, cells=11):
    """The first table row that starts with `first_cell` and has `cells` cells
    (the report has a short counts table and the score table)."""
    for line in report.splitlines():
        if line.startswith(f"| {first_cell}"):
            row = [c.strip() for c in line.strip("|").split("|")]
            if len(row) >= cells:
                return row
    return None


def main():
    need_cli()
    print(f"# Accuracy\n\nMachine: {machine()}\n")
    print("Scores are for addresses written one to a line unless noted. \"Coarse\" folds street parts")
    print("into the street name (docs/usaddress-comparison.md, \"Scoring rules\"); \"fine\" keeps them apart.\n")

    cases, ok, total, pct = corpus_report(ROOT / "tests" / "corpus")
    print("## Golden corpus (tests/corpus)\n")
    print(f"{ok} of {total} whole addresses right ({pct}%) over {cases} cases. These cases were written")
    print("alongside the parser, so this is a regression check, not an accuracy estimate.\n")

    ext = sorted((ROOT / "tests" / "external" / "usaddress").glob("*.xml"))
    if ext:
        rep = compare("gold", *map(str, ext))
        row = table_row(rep, "**All not in training**")
        print("## usaddress's test files, not in its training data\n")
        print("These are usaddress's own regression tests, so it is expected to score near 100%.\n")
        if row:
            print("| | Addresses | Tokens | Ours: tokens | usaddress: tokens | Ours: whole address | usaddress: whole address |")
            print("|---|---|---|---|---|---|---|")
            print("| all | " + " | ".join(row[1:7]) + " |\n")

    if (GEOCODE / "test_usaddress.xml").exists():
        rep = compare("gold", str(GEOCODE / "test_usaddress.xml"))
        row = table_row(rep, "test_usaddress.xml")
        print("## Geocode benchmark (blind, 30,622 one-line addresses)\n")
        if row:
            print("| | Ours | usaddress |\n|---|---|---|")
            print(f"| Tokens (coarse) | {row[3]} | {row[4]} |")
            print(f"| Whole address (coarse) | {row[5]} | {row[6]} |")
            print(f"| Tokens (fine) | {row[7]} | {row[8]} |")
            print(f"| Whole address (fine) | {row[9]} | {row[10]} |\n")
    else:
        print("## Geocode benchmark\n\nSkipped: `local-data/geocode-benchmark/` is not here.\n")

    if (FLORIDA_SAMPLE / "sample.jsonl").exists():
        cases, ok, total, pct = corpus_report(FLORIDA_SAMPLE)
        print("## Hand-labeled Florida sample (190 addresses)\n")
        print(f"{ok} of {total} whole addresses right ({pct}%), scored after the parser fixes that the sample's")
        print("errors led to (146 of the 150 uniformly drawn rows were right before them).\n")
    else:
        print("## Florida sample\n\nSkipped: `local-data/florida-sample/` is not here.\n")


if __name__ == "__main__":
    main()
