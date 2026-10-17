# Benchmarks

Accuracy, speed and size of doorplate, measured 2026-10-17. The scripts are in
`benchmarks/` (`benchmarks/README.md`) so every number can be rerun. usaddress
(the Python CRF library, 0.5.16) is one reference point, not the frame: it is
the tool people know, and the two libraries do different jobs (usaddress labels
tokens; doorplate also standardizes, builds a key and a slug, and reports
diagnostics).

This library formats and parses; it does not validate deliverability. None of
the figures below say an address exists or can receive mail.

**Machine:** Intel Core i9-9900K (14 threads), 49 GB RAM, Linux 6.18 on WSL2,
Rust 1.91, Python 3.13 (speed and size runs in a virtual environment),
doorplate 0.1.0 built with `--release`.

## In short

| | doorplate | usaddress |
|---|---|---|
| Geocode benchmark, whole address (coarse) | 96.4% | 88.2% |
| usaddress's own test files, whole address (coarse) | 99.2% | 100.0% |
| One address from Python, `parse` / `usaddress.parse` | 21 µs | 82 µs |
| 100,000 addresses, `parse_many` / a loop of `usaddress.parse` | 0.32 s | 8.2 s |
| One address in Rust (median of the corpus shapes) | 10 to 23 µs | |
| Installed size (wheel / package and dependencies) | 1.7 MB (4.1 MB unpacked) | 4.8 MB |
| Import time | 1 ms | 10 ms |

## Accuracy

Scores use usaddress's labels, folded into the coarse set unless marked fine
(`docs/usaddress-compat.md`, "Scoring rules"). A whole address counts as
right only when every token is. `benchmarks/accuracy.py` prints these tables.

### Geocode benchmark (30,622 addresses, one line, no commas)

The test split of the [geocoding address-parsing benchmark](https://github.com/zhengcongyin/geocoding-address-parsing-benchmark):
street addresses with input errors and variants mined from a production
geocoder's logs, written uppercase as `4394 MAIN ST PORT HENRY NY 12974`. It
states no license, so it stays in the gitignored `local-data/` and only
aggregate numbers are published.

| | doorplate | usaddress |
|---|---|---|
| Tokens (coarse) | 99.3% | 97.7% |
| Whole address (coarse) | 96.4% | 88.2% |
| Tokens (fine) | 95.9% | 94.7% |
| Whole address (fine) | 82.3% | 77.3% |

Neither library was trained on it. doorplate's rules and place list were written
after reading this benchmark's error classes, so read 96.4% as an estimate for data
of this shape (one line, US street addresses, a city and state), not as a held-out
result for any address you may have.

### usaddress's own test files

Addresses that are not in usaddress's training files (2,248 of 9,088).

| | Addresses | Tokens | doorplate: tokens | usaddress: tokens | doorplate: whole | usaddress: whole |
|---|---|---|---|---|---|---|
| all | 2,248 | 12,629 | 99.6% | 100.0% | 99.2% | 100.0% |

These are usaddress's own regression tests, so its near-100% is expected. Read
the gap as where the two libraries choose differently (hard cases in
`labeled.xml`, 89.8% for doorplate, and labels doorplate does not model), not as
an accuracy contest.

### Golden corpus

437 of 437 (100%). The corpus is written alongside the parser and every bug
fix adds a case, so it is a regression check and says nothing about accuracy on
new data.

## Speed

### Rust (criterion, `crates/doorplate/benches/parse.rs`)

Median of a measurement run, one thread unless noted.

| Benchmark | Time |
|---|---|
| `parse`, street only (`1305 SUNWOOD DR`) | 10.0 µs |
| `parse`, PO box | 10.0 µs |
| `parse`, military | 10.3 µs |
| `parse`, highway | 16.7 µs |
| `parse_fields` (street, city, state, ZIP) | 17.8 µs |
| `parse`, unit and ZIP+4 | 20.7 µs |
| `parse`, full one line | 23.2 µs |
| The 437 corpus inputs, one by one | 9.6 ms (22 µs each) |
| The same, `parse_many` (rayon, 14 threads) | 2.4 ms (5.5 µs each) |
| `to_usps` | 0.71 µs |
| `to_reso` | 1.38 µs |
| `key` | 0.37 µs |
| `slug` | 0.91 µs |

Parsing is by far the biggest cost; the emitters are about a tenth of a parse
or less, and a `ParsedAddress` builds them only when asked.

The command-line tool: `doorplate parse-file` over 440,288 distinct Florida
public-record addresses (line 1, line 2, city, state and ZIP columns) took 15.4 s,
28,600 rows per second on one thread, including reading the CSV and writing
about 30 columns per row.

### Python

`benchmarks/speed_python.py`, 100,000 addresses (the inputs repeated), best of
five runs.

| | Corpus inputs | Geocode addresses |
|---|---|---|
| doorplate `parse` | 21 µs | 36 µs |
| doorplate `parse`, then `key()` and `to_usps()` | 24 µs | 39 µs |
| doorplate `tag` | 24 µs | 38 µs |
| doorplate `parse_tokens` | 23 µs | 38 µs |
| doorplate `parse_many` (14 threads) | 3.2 µs | 4.9 µs |
| usaddress `parse` | 82 µs | 95 µs |
| usaddress `tag` | 91 µs | 99 µs |

Call by call on one thread, `parse_tokens` is 3.6x `usaddress.parse` on the
corpus inputs and 2.5x on the geocode addresses. `parse_many` parses on all
cores with the GIL released: 26x and 19x `usaddress.parse`, which is
single-threaded (usaddress could be run in several processes; that is a
different comparison). A Python call costs about what the Rust parse does
(21 µs against 22 µs on the corpus); the boundary is not the bottleneck.

Building DataFrames (`crates/doorplate-py/benchmarks/bench_batch.py`, 100,000
corpus addresses): a loop of `parse` 2.11 s, `parse_many` 0.32 s (6.6x),
`parse_many` into a pandas frame of seven columns 0.49 s (4.3x), into a Polars
frame 0.40 s (5.3x). Reading attributes from 100,000 Python objects is now
most of the time; `to_dict()` on every row costs about as much as parsing again.

## Size and memory

`benchmarks/size_memory.py`.

| | Size |
|---|---|
| `doorplate` CLI binary (release, as built) | 4.8 MB (4.4 MB stripped) |
| `doorplate` wheel (abi3, one wheel per platform) | 1.7 MB |
| The extension module inside it | 4.1 MB |
| `usaddress` and its dependencies, installed | 4.8 MB (the model is 134 KB; the rest is python-crfsuite) |

The lexicon tables, the national place list and the Florida pack are compiled
in, so there is no model or data file to download or load. The tables are
inside the crate, so the crate package (47 files, 3.6 MiB unpacked,
0.8 MB compressed) and the Python source distribution (0.8 MB) carry them and
build from a clean directory.

Peak resident memory of a fresh Python process parsing 100,000 addresses
(corpus inputs repeated):

| | Import | After import | After the batch |
|---|---|---|---|
| Python alone | | 16 MB | |
| doorplate, loop of `parse`, each result dropped | 1 ms | 16 MB | 18 MB |
| usaddress, loop of `parse`, each result dropped | 10 ms | 18 MB | 20 MB |
| doorplate, loop of `parse`, 100,000 results kept | 1 ms | 16 MB | 151 MB |
| usaddress, loop of `parse`, 100,000 results kept | 10 ms | 18 MB | 126 MB |
| doorplate, `parse_many`, 100,000 results kept | 1 ms | 16 MB | 240 MB |

The parser itself adds about 2 MB to a process either way. A `ParsedAddress`
holds every component and its source spans (about 1.4 KB each against about
1.1 KB for usaddress's token list). `parse_many` builds all its results at once
on several threads, so its peak is higher than a loop's. usaddress loads its
model on its first call, so its memory after import understates it.

## What these numbers do not say

- They do not say an address is real or deliverable.
- Accuracy is for US addresses written in English, and mostly one line. The
  geocode benchmark has street addresses only (no units, PO boxes or routes);
  rural routes, military and general delivery are covered by the corpus, not by
  a measured rate.
- Both libraries' scores depend on the label set. usaddress labels more things
  (recipient names, landmarks, corners) that doorplate leaves unparsed on
  purpose, and those tokens are out of scope in the scoring.
- The speed figures are for one machine with the GIL released where noted.
  Relative numbers hold up better than absolute ones.
- Confidence scores are not measured: `confidence` is always `None`.

## Rerunning

```sh
cargo build --release -p doorplate-cli
(cd crates/doorplate-py && maturin develop --release)
python benchmarks/accuracy.py
python benchmarks/speed_python.py
python benchmarks/speed_rust.py [--csv FILE --column NAME ...]
python benchmarks/size_memory.py
```

`benchmarks/README.md` has the setup and what each script needs. The geocode
benchmark is not redistributable, so it is not in the repository: download it
and put it in `local-data/geocode-benchmark/` (`benchmarks/README.md`); a script
skips a source that is not there.
