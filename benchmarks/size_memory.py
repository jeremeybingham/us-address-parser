#!/usr/bin/env python3
"""Size, import time and memory: doorplate against usaddress.

    python benchmarks/size_memory.py

Needs the release CLI, `maturin` (to build a release wheel), the installed
`doorplate` extension and `uv` (to install usaddress into a scratch directory
and measure it). Prints Markdown. Memory is the peak resident set size of a
fresh Python process (`ru_maxrss`), which is what a worker holding the parser
costs; it includes the interpreter.
"""
import json
import shutil
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from _common import CLI, ROOT, corpus_inputs, machine, need_cli, run  # noqa: E402


def mb(n):
    return f"{n / 1048576:.1f} MB"


def tree_size(path):
    return sum(f.stat().st_size for f in Path(path).rglob("*") if f.is_file())


def child(code):
    """Run `code` in a fresh Python; it prints a JSON object."""
    return json.loads(run([sys.executable, "-c", code]))


MEASURE = """
import json, resource, sys, time
def rss(): return resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1024
base = rss()
t = time.perf_counter()
{imp}
imp_s = time.perf_counter() - t
after_import = rss()
texts = {texts!r}
texts = (texts * ({n} // len(texts) + 1))[:{n}]
{work}
print(json.dumps(dict(base=base, import_s=imp_s, after_import=after_import, after_batch=rss())))
"""


def memory(imp, work, n=100_000):
    texts = corpus_inputs()
    return child(MEASURE.format(imp=imp, work=work, texts=texts, n=n))


def main():
    need_cli()
    print(f"# Size and memory\n\nMachine: {machine()}\n")

    # Binary and wheel sizes.
    cli = CLI.stat().st_size
    stripped = None
    if shutil.which("strip"):
        with tempfile.TemporaryDirectory() as d:
            copy = Path(d) / "doorplate"
            shutil.copy(CLI, copy)
            subprocess.run(["strip", str(copy)], check=True)
            stripped = copy.stat().st_size
    with tempfile.TemporaryDirectory() as d:
        run(["maturin", "build", "--release", "-o", d, "-m", str(ROOT / "crates/doorplate-py/Cargo.toml")])
        wheel = next(Path(d).glob("*.whl"))
        wheel_size = wheel.stat().st_size
        with zipfile.ZipFile(wheel) as z:
            so = sum(i.file_size for i in z.infolist() if i.filename.endswith(".so"))
            unpacked = sum(i.file_size for i in z.infolist())
    # What would be published: the crate package and the Python source distribution.
    run(["cargo", "package", "-p", "doorplate", "--allow-dirty", "--no-verify"], cwd=ROOT)
    crate = next((ROOT / "target" / "package").glob("doorplate-*.crate")).stat().st_size
    with tempfile.TemporaryDirectory() as d:
        run(["maturin", "sdist", "-o", d, "-m", str(ROOT / "crates/doorplate-py/Cargo.toml")])
        sdist = next(Path(d).glob("*.tar.gz")).stat().st_size
    print("## Size\n")
    print("| | Size |\n|---|---|")
    print(f"| `doorplate` CLI binary (release, as built) | {mb(cli)} |")
    if stripped:
        print(f"| `doorplate` CLI binary, stripped | {mb(stripped)} |")
    print(f"| `doorplate` wheel (abi3, this platform) | {mb(wheel_size)} |")
    print(f"| the extension module inside it | {mb(so)} |")
    print(f"| the wheel unpacked | {mb(unpacked)} |")
    print(f"| `doorplate` crate package (`.crate`, compressed; carries the tables) | {mb(crate)} |")
    print(f"| `doorplate` Python source distribution (sdist) | {mb(sdist)} |")

    # usaddress and its dependencies, installed into a scratch directory.
    with tempfile.TemporaryDirectory() as d:
        subprocess.run(["uv", "pip", "install", "-q", "--target", d, "usaddress"], check=True)
        print(f"| `usaddress` and its dependencies, installed | {mb(tree_size(d))} |")
        model = next(Path(d).rglob("usaddr.crfsuite"), None)
        if model:
            print(f"| the usaddress model file | {mb(model.stat().st_size)} |")
    print("\nThe lexicon tables, the national place list and the Florida pack are compiled into the binary\n"
          "(`docs/places.md` sizes the place list); there is no model or data file to load.\n")

    # Import time and memory.
    base = memory("pass", "pass")
    ours = memory("import doorplate as m", "m.parse_many(texts)")
    ours1 = memory("import doorplate as m", "[m.parse(t) for t in texts]")
    theirs = memory("import usaddress as m", "[m.parse(t) for t in texts]")
    ours_d = memory("import doorplate as m", "for t in texts: m.parse(t)")
    theirs_d = memory("import usaddress as m", "for t in texts: m.parse(t)")
    print("## Import time and memory\n")
    print("Peak resident set size of a fresh Python process, parsing 100,000 addresses (corpus inputs repeated).\n")
    print("| | Import | RSS after import | RSS after the batch |\n|---|---|---|---|")
    print(f"| Python alone | | {base['after_import']:.0f} MB | |")
    print(f"| `doorplate`, `parse_many` (all results kept) | {ours['import_s'] * 1000:.0f} ms | {ours['after_import']:.0f} MB | {ours['after_batch']:.0f} MB |")
    print(f"| `doorplate`, a loop of `parse` (all results kept) | {ours1['import_s'] * 1000:.0f} ms | {ours1['after_import']:.0f} MB | {ours1['after_batch']:.0f} MB |")
    print(f"| `usaddress`, a loop of `parse` (all results kept) | {theirs['import_s'] * 1000:.0f} ms | {theirs['after_import']:.0f} MB | {theirs['after_batch']:.0f} MB |")
    print(f"| `doorplate`, a loop of `parse` (each result dropped) | {ours_d['import_s'] * 1000:.0f} ms | {ours_d['after_import']:.0f} MB | {ours_d['after_batch']:.0f} MB |")
    print(f"| `usaddress`, a loop of `parse` (each result dropped) | {theirs_d['import_s'] * 1000:.0f} ms | {theirs_d['after_import']:.0f} MB | {theirs_d['after_batch']:.0f} MB |")
    print("\nNote: usaddress loads its model on the first call, so its memory after import understates it; the\n"
          "batch column includes it. \"Kept\" rows hold 100,000 result objects; \"dropped\" rows show what the\n"
          "parser itself needs. `parse_many` builds all its results at once on several threads, so its peak is\n"
          "higher than a loop's.\n")


if __name__ == "__main__":
    main()
