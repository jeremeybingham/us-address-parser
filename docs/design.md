# US Address Parser: Design and Plan

A Rust-core US address parsing library with a Python wrapper. This document
records the goals, the architecture decisions, the milestone plan and its
progress, and the open tasks and decisions. Detailed specs live beside it:

| Document | What it holds |
|---|---|
| `docs/usps-pub28-notes.md` | Sources for every lexicon table, parser rule and USPS emitter rule |
| `docs/key-format.md` | The `k1` key (a public contract) and the slug |
| `docs/reso-mapping.md` | RESO Data Dictionary 2.0 field mapping and decisions |
| `docs/usaddress-comparison.md` | Comparison with usaddress, the blind benchmark, the accuracy backlog |
| `docs/places.md` | The national place list: sources, licenses, how it is built and compiled |
| `docs/packs/fl.md` | The Florida pack: sources, licenses, evidence, what is left |
| `docs/future-sources.md` | Outside address sources evaluated (NAD, OpenAddresses, Netsyms, Kaggle), what v1's accuracy numbers cover, and the deferred v2 plan |
| `docs/prior-art.md` | Lessons from the author's Florida address code |
| `tests/corpus/README.md` | Golden-corpus record format and labeling conventions |
| `crates/doorplate-py/README.md` | The Python package: API, batch and dataframe recipes, usaddress mapping |
| `examples/` | Notebook tour, Polars and pandas pipelines and invented sample inputs (item 59); run by `crates/doorplate-py/tests/test_examples.py` |
| `docs/repository_cleaning.md` | Text changes deferred to the fresh repository before the first release (decision 42) |
| `CHANGELOG.md` | What changed, newest first (everything is "Unreleased" until item 46) |

**Status (2026-10-16):** Milestones 0 to 6 are done. The Rust library and
the `doorplate` CLI parse, emit USPS Pub 28 and RESO output, build the `k1` key
and the slug, and report diagnostics; a national place list (50 states, DC,
Puerto Rico) and a curated Florida pack find cities, and `default_state`
hints the state for input that names none. The Python wrapper
(`doorplate`, abi3, `parse`/`parse_fields`/`parse_many`/`tag`/
`parse_tokens`) is built and tested, with wheels for Linux, macOS and
Windows built on every pull request. Measures: golden corpus 428 cases, all
passing; blind geocode benchmark 96.4% of whole addresses (coarse) against
usaddress's 88.2%; 27 Python tests. No CRF will be built (decision 32).
Nothing is published yet.
**Next (section 10): the release.** Milestone 7 is closed: the tagger
records its uncertain decisions (item 37) and the hand-labeled Florida sample
found and fixed seven rules (item 36). The calibrated confidence score
(item 38) is **deferred past the first release** (decision 43): the data
that exists supports only a coarse score, so `Diagnostics::confidence` stays
`None` in the first release. Milestone 8's parser and hardening work is done:
the unused `model_version` and `crf` removed (item 53), `country_ignored`
(item 54), `intersection_detected` (item 55), the Netsyms fixes (item 56) and
fuzz targets (item 43). The last two Florida sample errors are fixed.
**The release plan is decided (decisions 44, 45 and 46):** the dataframe
helpers and examples (items 50 and 59) are done, and the name is `doorplate`
(item 58, decision 46). The rename (45) comes next, then the Pub 28 recheck
(44), benchmarks (57), the text cleaning in `docs/repository_cleaning.md`
(61), a fresh repository with no history (62) and publishing from it (46). The custom layer (items 47 to 49)
and its guide (60) are deferred past the first release (decision 45).
Section 10 is the plan, in build order, one pull request per item; section
13 has the decisions and the unscheduled backlog.

**Added 2026-10-16 (usaddress feature review):** a feature comparison with
usaddress (`docs/usaddress-comparison.md`, "Feature comparison") found gaps
in the Python surface, in customization, and in a few input and output
shapes. They are now built or scheduled: `tag()`, `address_type` and
`default_state` are done (Milestone 6, items 30 to 32); the trailing country
and intersections are Milestone 8 items 54 and 55, done (decisions 36 and 37); a
runtime **custom layer** (section 11, "Custom layer"; decision 22; deferred
past the first release, decision 45) and optional dataframe helpers are
Milestone 9; display and long forms stay open
in section 13.

---

## 1. Goal and scope

Build a small, dependable library that:

1. Parses US address strings (and pre-split address fields) into components.
2. Emits normalized output in **USPS Publication 28** style and **RESO Data Dictionary** field names.
3. Produces **database-ready identifiers**: a stable canonical match key and a URL-safe slug.
4. Reports **diagnostics** (warnings, unparsed remainder) so bulk users can route bad rows to review.
5. Is usable from Rust directly and from Python via a PyO3 wrapper, with no C library or data download required at runtime.

The author's Florida address code (auction-tracker) is the main
source of domain knowledge; what carried over is in `docs/prior-art.md`.

### Explicit non-goals (v1)

- **Address validation or deliverability.** Pub 28 formatting is not the same as a deliverable address; real validation needs CASS-certified data. The README states this plainly.
- Geocoding, fuzzy matching, address correction, international addresses.
- Correcting addresses. The place list (Milestone 5) can fill a missing city from the ZIP or flag a city that disagrees with its ZIP, but the parsed `city` stays what the input said; the place match is reported beside it (section 13, decision 13).
- Runtime-configurable *shipped* lexicons: the Pub 28 tables and the state
  packs are fixed at compile time. User customization is a separate,
  add-only **custom layer** (section 11; deferred, decision 45), not an edit
  of those tables.

---

## 2. Research summary (October 2026)

Findings from web research during planning, not from reading source code in
depth.

### Rust ecosystem

| Project | What it is | Assessment |
|---|---|---|
| `us-addrs` (raphaellaude) | Rust port of Python `usaddress`; claims ~5x faster | Closest prior art. Self-described as under active development, accuracy not yet matching usaddress. Not production-grade. |
| `us_address_parser` | Simple rule-based crate | Street number, direction, name, type, unit only. No city/state/ZIP. |
| `rustpostal`, `postal` (rust-postal), `libpostal-rs`, `libpostal-sys` | Bindings to libpostal (C) | Heavy: needs the C library and large data files; libpostal is not thread-safe; international and statistical, not tuned for USPS conventions. |
| `crfs` | Pure-Rust port of CRFsuite | **Inference only** (no training). A building block for running a usaddress-style CRF model in pure Rust. |

Conclusion: no mature pure-Rust US address parser with Python bindings exists.

### Python ecosystem (what we compete with)

- `usaddress` (datamade): the standard. CRF model via parserator and python-crfsuite. MIT license.
- `usaddress-scourgify`: normalization to USPS Pub 28 and RESO guidelines; does no validation.
- libpostal Python wrappers: require the libpostal C library.

### Rust core + Python wrapper pattern

Well proven (polars, pydantic-core, orjson, tokenizers), built and published
with PyO3 and maturin. Per-call boundary overhead can erase speed gains for
single addresses, so expose batch APIs and release the GIL.

**Accuracy, not speed, is the bar to beat.** The comparison with usaddress
and a blind benchmark are in `docs/usaddress-comparison.md`.

---

## 3. Architecture decisions

Items the author accepted as defaults are marked *(default)*; revisit if
they cause friction.

| # | Decision | Resolution |
|---|---|---|
| 1 | Parsing approach | Rules plus the place list. No CRF fallback: decided no-go after the place list (item 29; section 13, decisions 10 and 32). The empty `crf` cargo feature was removed (item 53). |
| 2 | Model training | Not applicable: no model is trained or shipped (decision 32). Was: train in Python on usaddress's MIT-licensed data, Rust inference via `crfs`. The unused `model_version` and `crf` feature were removed (decision 35, item 53). |
| 3 | Input scope | Both single-line strings and multi-field input (line1, line2, city, state, zip). |
| 4 | Geographic scope | US-wide grammar. Place data in layers (section 11): a national place list generated from public data, plus curated state packs, Florida first (Milestone 5). Other pack contents (road patterns) are deferred. |
| 5 | Canonical key | Unit inclusion is an explicit option (default off, property level); the key carries a version prefix. City and ZIP+4 excluded. Spec in `docs/key-format.md`. |
| 6 | Error model | Always return a `ParsedAddress` with diagnostics; `Result` only for unusable input (empty). |
| 7 | Lexicon extensibility | Shipped lexicons and state packs are fixed at compile time. Users customize through an add-only runtime **custom layer** in the state-pack format (section 11; decision 22), deferred past the first release (decision 45). |
| 8 | Data sources | USPS Pub 28 and the RESO Data Dictionary; provenance of each table documented. |
| 9 | Python surface *(default)* | Done: typed objects plus `to_dict()`, and a usaddress-shaped `tag()` with `address_type` (decision 23). No pandas/Polars dependency in the wheel; tested recipes in the README, and optional dataframe helpers come later as extras (decision 26). |
| 10 | Distribution *(default)* | abi3 wheels (built on every pull request for Linux, macOS and Windows); library and key-format versions tracked independently. Names (decision 46): `doorplate` on PyPI and crates.io, `doorplate-cli` for the CLI crate, binary `doorplate` with `addr` as an alias; decision 33's names are replaced. |
| 11 | Compatibility | Own schema; optional usaddress-compatible output, both done: per-token labels (`to_usaddress_tags()`) and joined components with an address type (`tag()`; decision 23). |
| 12 | License | MIT OR Apache-2.0 (confirmed 2026-10-15). |
| 13 | Component spans | Byte ranges into the **original** input, not the cleaned text. `clean.rs` keeps a map from cleaned to original offsets. For multi-field input a span also names its field. |
| 14 | Multi-field input | `parse_fields` tags each field on its own, using the field as a strong signal (the city field is the city). When city, state and ZIP are all empty, `line1` and `line2` are read like one line (a street line plus a `CITY, ST ZIP` line). |
| 15 | Address shapes with no field yet | `ParsedAddress` has fields for highway routes (prefix apart from the route number), general delivery, military (PSC/CMR/UNIT box plus APO/FPO/DPO) and a second unit. The RESO emitter folds them into `StreetName` or `UnitNumber` (`docs/reso-mapping.md`). |
| 16 | Key shape | Pipe-delimited uppercase ASCII with a one-letter form (`S`, `P`, `R`, `M`, `G`). `k1:` needs state and ZIP5; otherwise `k1n:`. Unit type is dropped at unit level so `Apt 4` equals `#4`. Placeholders and addresses with nothing to key on give `None`. |

Later decisions, made while building, are in section 13.

---

## 4. Repository layout

As built; items marked *(planned)* do not exist yet.

```
Cargo.toml                  # workspace: doorplate, doorplate-cli, doorplate-py
rust-toolchain.toml, .cargo/config.toml   # stable toolchain; MSRV-aware resolver (MSRV 1.80; doorplate-py needs 1.83)
README.md, CLAUDE.md, CHANGELOG.md, NOTICE, LICENSE-MIT, LICENSE-APACHE
docs/                       # this document and the specs listed at the top
crates/
  doorplate/                # pure Rust, no Python dependency
    build.rs                # compiles data/*.toml into static tables
    src/                    # clean, tokenize, classify, tagger/, assemble, parser,
                            # input, place, lexicon, types, emit/{key,usps,reso,slug,usaddress}, corpus
    tests/                  # corpus.rs, no_panic.rs (proptest), usaddress_gold.rs
  doorplate-cli/                 # `doorplate` binary (alias `addr`): parse, parse-file, corpus test
  doorplate-py/                  # PyO3 bindings: src/lib.rs, python/doorplate/ (package, stubs),
                            # tests/ (pytest), benchmarks/
data/                       # suffixes, directionals, units, states, special (.toml)
  places/                   # the national place list (generated), compiled behind `places-us`
  packs/fl/                 # the Florida pack, compiled behind `state-fl` (section 11)
benches/parse.rs            # criterion benches
tests/
  corpus/                   # golden corpus (JSONL) and passing.txt baseline
  external/usaddress/       # usaddress's labeled test files (MIT, vendored)
tools/
  corpus/emit_expect.py     # writes emitter expectations into corpus cases from the specs
  compare/                  # usaddress comparison (gold, raw, tag modes) and training-overlap scripts
  packs/                    # Florida pack export, observed-ZIP and unmatched-city scripts, dbpr.py reader
  places/                   # build_us.py: national place list from Census and GeoNames data
  calibrate/                # Florida-sample labeling (item 36) and an exploratory score fit (item 38, deferred)
.github/workflows/ci.yml    # fmt, clippy, test, corpus report; Python tests (3.10, 3.13)
.github/workflows/wheels.yml # abi3 wheels for Linux, macOS, Windows, built and tested on every PR
local-data/                 # gitignored: private and unlicensed test data, never committed
```

---

## 5. Core design (`doorplate`)

### Pipeline

```
raw input
  -> clean.rs     whitespace, accents, Unicode punctuation, uppercase; byte map to the original
  -> tokenize.rs  tokens with original spans, lines and comma segments, glued-token splits
  -> classify.rs  token shapes and longest-match lexicon lookups
  -> tagger/      rules and place lookups
  -> assemble.rs  ParsedAddress, component spans, warnings
  -> emit/        read-only consumers: key, usps, reso, slug, usaddress tags
```

Emitters never mutate the parsed struct and never re-parse.

### Principles

- **Deterministic.** Same input, same output, across platforms.
- **Spans preserved.** Components keep their source byte ranges (`SourceInfo::spans`).
- **Never panic on input.** Covered by proptest properties (`tests/no_panic.rs`); fuzz targets (`fuzz/`, item 43) cover the public parse and emit paths.
- **Lexicon access behind one interface** (`Lexicons`, built from `data/*.toml`, the national list and the packs at build time). The state packs and the deferred custom layer (decision 45) plug in here.
- No global mutable state; `Parser` is cheap and `Send + Sync`.

### `ParsedAddress`

Defined in `crates/doorplate/src/types.rs`. Fields: street number,
pre/post directional, street name, suffix (`StreetSuffix`), `highway`
(state, prefix, route), first and second unit (type and number), `po_box`,
`rural_route` (RR/HC, route, box), `general_delivery`, `military` and
`military_post_office`, `attention`, `building`, city, state (`StateCode`),
ZIP5, ZIP+4, `place` (the place the city stands for, beside the city;
decision 13), `diagnostics` and `source`.

`Diagnostics` holds the warnings (`Warning`, each with a stable `code()`
string; `Warning::ALL` lists them), the unparsed remainder, the library
version and `confidence` (reserved: always `None` in the first release,
decision 43; intended meaning in section 13, decision 12). There is no `model_version`: no model is shipped (decision 35).

### Public API

```rust
let parser = Parser::new();
let a = parser.parse("123 north main street apt 4, Miami FL 33101")?;  // Err only for unusable input
let b = parser.parse_fields(&Fields { line1, line2, city, state, zip })?;
let all = parser.parse_many(&inputs);   // parallel with the `rayon` feature; keeps input order

a.to_usps();             // UspsAddress; to_usps_wrapped(max_line); lines() in mailpiece order
a.to_reso();             // ResoAddress with Data Dictionary 2.0 names; fields(), length_overruns()
a.key(KeyOptions::property());   // Option<String>, "k1:S|123|N|MAIN|ST||FL|33101"
a.key(KeyOptions::unit());
a.slug();                // Option<String>, display only
a.to_usaddress_tags();   // tokens with usaddress labels
```

Added in Milestone 6 (decisions 23 and 24), and one planned addition:

```rust
a.tag();                 // TaggedAddress: joined components keyed by usaddress label
a.address_type();        // AddressType: Street, PoBox, RuralRoute, Military, GeneralDelivery, Dual, Unknown
let p = Parser::with_options(ParseOptions { default_state: Some(state) })?;
let p = Parser::new().with_custom_layer(CustomLayer::from_toml(&text)?);   // deferred (decision 45); not in the first release
```

`tag()` is the counterpart of `usaddress.tag()`: tokens that share a label
are joined with a space in input order (`StreetName` for `MARTIN LUTHER KING`),
and unlike usaddress it never fails on a repeated or split label (no
`RepeatedLabelError`); a label that appears in two separate runs is joined
and a warning (`repeated_label`) says so. `address_type()` is the finer
classification; `TaggedAddress::usaddress_type()` gives usaddress's own three
values (`Street Address`, `PO Box`, `Ambiguous`), worked out by its rule (an
`AddressNumber` first, then a `USPSBoxID`) so a dual address is a Street
Address and a rural route a PO Box, as there. Both are built from the parsed fields
and spans by an emitter, so they follow the emitter rule (read-only, no
re-parsing).

`ParseOptions::default_state` is a hint for lookups only
(decision 24): when the input names no state and has no known ZIP, a city is
looked up in that state (`281 EVELYN DR DOTHAN`, with no commas, finds
DOTHAN), and the state's pack readings apply (`CR` is CIRCLE, `SR` is STATE
ROAD in Florida). It never fills `state`, so `missing_state` still fires and
the key keeps its `k1n` form; `default_state_applied` says when it decided a
place or a reading. It takes a USPS code or a state name and fails with
`OptionsError` for anything that is not a state, DC or territory (so not a
military `AE`).

Cargo features: `rayon` (parallel `parse_many`), `corpus` (the golden-corpus
runner, pulls in serde), `places-us` and `state-fl` (the place data, both on by
default; section 11). The default build
has no runtime dependencies. When built (deferred, decision 45), the custom layer's loader (`CustomLayer::from_toml`) takes text
and adds only a TOML parser dependency under a `custom-layer` feature; the
core never reads files, so the CLI and the Python wrapper do the file I/O.

---

## 6. Emitters and identifiers

All four are built. Rules and sources: USPS in `docs/usps-pub28-notes.md`,
RESO in `docs/reso-mapping.md`, key and slug in `docs/key-format.md`.

- **USPS** (`to_usps`): Pub 28 style lines. Layout rules were checked
  against the live Pub 28 text in October 2026.
- **RESO** (`to_reso`): Data Dictionary 2.0 Property address fields. The
  mapping choices were reviewed and accepted by the author on 2026-10-15.
- **Key** (`key`): deterministic, for dedupe and joins. "123 N. Main Street,
  Apt 4" and "123 north main st #4" give the same key. A format change means
  a new prefix (`k2:`), never an in-place edit.
- **Slug** (`slug`): URL-safe, lowercase, hyphenated, built from the USPS
  output. Display only; not unique, not versioned.

**Key stability (confirmed).** The key uses street components, state and
ZIP5, not the city: city names are noisy and the field most likely to change
as tuning improves, so leaving them out keeps stored keys stable. Addresses
missing a state or ZIP get the weaker `k1n:` prefix and a warning. The slug
does include the city.

---

## 7. Python wrapper (`doorplate-py`, done)

Package `doorplate` (decision 46), the
extension module `doorplate._core`: PyO3 0.30 with maturin, **abi3
wheels**, Python 3.10 and later, no runtime dependencies. API and recipes:
`crates/doorplate-py/README.md`.

- `parse(text)`, `parse_fields(...)` and **`parse_many(texts)`** (a list or
  tuple; releases the GIL and parses in parallel with Rust threads; `None` for
  an element with no address text), each with an optional `default_state`; a
  `Parser(default_state=...)` class holds the setting.
- `ParsedAddress` is read-only and typed (stubs in `__init__.pyi`, `py.typed`):
  the components, `place`, `warnings`, `unparsed`, `confidence`, and the
  emitters `key(unit=False)`, `slug()`, `to_usps(max_line=None)`, `to_reso()`,
  `to_usaddress_tags()`, `tag()`, `address_type()`, `repeated_labels()` and
  `to_dict()`. `highway`, `rural_route`, `military` and `place` are plain dicts.
- **usaddress-shaped functions:** `tag(text)` returns `(dict, type)` with
  usaddress's label and type names; `parse_tokens(text)` returns
  `[(token, label)]`. No `RepeatedLabelError`: the runs are joined and
  `repeated_labels()` lists the labels (decision 23). Differences are in the
  repository README, "Coming from usaddress".
- `ParseError` (a `ValueError`) only for input with no address text; a
  `default_state` that is not a state raises `ValueError`.
- `confidence` is `float | None`: always `None` in the first release
  (decision 43); the field is kept so that adding the score later changes no
  signature.
- Deferred (decision 45): `Parser(custom_layer=path_or_text)`. Not in the first release.
- No pandas or Polars dependency. The README has tested recipes for both
  (Polars first; 0.34 s against 0.45 s to build a 7-column frame from 100,000
  addresses). Optional helpers come later (Milestone 9, item 50; decision 26)
  as pure-Python extras that call `parse_many`. Multi-field input has no batch
  call (loop over `parse_fields`, or join the columns).

---

## 8. CLI (`doorplate-cli`, done)

- `doorplate parse "<address>"` prints one JSON record (components, warnings, key, slug, USPS and RESO output); `--pretty`, `--usps`, `--usaddress`, `--tag`, field options (`--line1` ... `--zip`), or one address per stdin line.
- `doorplate parse-file IN.csv --column NAME [--line2 ...] [-o OUT.csv]` appends `addr_`-prefixed columns.
- `doorplate corpus test [--dir DIR] [--verbose] [--update-baseline]` runs the golden corpus and prints the accuracy report.
- `--default-state ST` on `parse` and `parse-file` (decision 24). Deferred (decision 45): `--custom-layer PATH` on `parse`, `parse-file` and `corpus test`, plus `doorplate layer check PATH` to validate a layer file (item 49).

---

## 9. Testing and quality

| Check | Status |
|---|---|
| **Golden corpus** (`tests/corpus/*.jsonl`): expected components and warnings; most cases also carry key, slug, USPS and RESO expectations written from the specs by `tools/corpus/emit_expect.py`. The runner fails on malformed records and on regressions against `passing.txt`. | Done: 428 cases, all passing (the runner also checks `place`, `address_type`, `tag` and `default_state` cases). |
| **USPS round trip**: parsing the USPS output of a parsed address gives the same key. | Done (`tests/corpus.rs`). |
| **Property tests** (proptest): no panic, valid spans, every emitter, `parse_many` consistency. | Done (`tests/no_panic.rs`). |
| **usaddress gold score**: this parser on usaddress's labeled test files, with a floor. | Done (`tests/usaddress_gold.rs`). |
| **Benchmarks** (criterion): single shapes, the corpus through `parse` and `parse_many`, the emitters. | Done (`benches/parse.rs`). |
| **CI**: `cargo fmt --check`, `clippy -D warnings`, `cargo test --workspace --all-features`, corpus accuracy report, the emitter-expectation check; a Python job (maturin develop, pytest on 3.10 and 3.13). | Done (`ci.yml`). |
| **Fuzzing** (cargo-fuzz, `fuzz/`, nightly): `parse` (with a `default_state` hint) and `parse_fields`, each running every emitter on the result. The tokenizer and assembler are `pub(crate)`, so they are reached through the public entry points. | Done (item 43); a short smoke run in CI. |
| **Python tests** (pytest): the wrapper, type stubs, `parse_many` (identical to a loop, GIL released), `tag` against usaddress, pandas and Polars recipes: 27 tests. **Wheels**: abi3 wheels built and tested on Linux x86_64 and aarch64, macOS universal2 and Windows x64, on Python 3.10 and 3.13. | Done (`crates/doorplate-py/tests`, `wheels.yml`). |
| **Blind accuracy**: labels neither this parser nor usaddress was tuned on. | Geocode benchmark run locally (`docs/usaddress-comparison.md`); a hand-labeled Florida sample is still open. |

The corpus is hand-labeled and the rules were tuned against it, so 100% on
it says little about unseen data; the blind benchmark is the better measure.

---

## 10. Milestones

Each milestone ends with passing tests. Each numbered item is one pull
request (its commits listed where it has more than one), small enough to
review on its own and leaving `main` green. Milestones 0 to 6 are done; the
open items are 44 to 46, 57, 58, 61
and 62 (Milestone 8; Milestone 7 is closed and its item 38 deferred,
decision 43), and 47 to 51, 59 and 60 (Milestone 9). **Build order
(decision 44):** 47, 48, 49, 50, 59, 60, then 44, 57, 58, 45, 61, 62, 46.
Items 36 and 37 are done, item 38 is deferred (decision 43) and item 51 is
not in the first release. Item 58 (the name, the author's research) may run
beside the Milestone 9 items. An item marked **(author)** needs the author's data or
decision before it can start; **(any time)** has no dependency and can go
in wherever it fits.

### Milestone 0: scaffolding (done)

1. Workspace `Cargo.toml`, `rust-toolchain.toml`, `.gitignore`, licenses
2. `README.md` (with the non-goals and the not-a-validator statement)
3. `CLAUDE.md`
4. `crates/doorplate` skeleton
5. `.github/workflows/ci.yml`

### Milestone 1: data and types (done)

6. `data/*.toml`, each with its source; verified against the live Pub 28 pages (`docs/usps-pub28-notes.md`)
7. `crates/doorplate/build.rs` (compile and validate the TOML into static tables)
8. `src/types.rs`
9. `src/lexicon.rs`

### Milestone 2: parser (done)

10. `src/clean.rs`
11. `src/tokenize.rs`
12. `src/classify.rs`
13. `src/tagger/mod.rs`, `src/tagger/rules.rs`
14. `src/assemble.rs`
15. `src/input.rs` (`Fields`)
16. `src/parser.rs` (`Parser`, `parse`, `parse_fields`, `parse_many`)

### Milestone 3: emitters (done)

17. `src/emit/key.rs`, `usps.rs`, `reso.rs`, `slug.rs`
18. `docs/usps-pub28-notes.md`, `docs/reso-mapping.md`, `docs/key-format.md`

### Milestone 4: corpus and CLI (done)

19. Golden corpus seeded from auction-tracker's tests and golden CSVs, Pub 28's own examples and regression cases. The author's own Florida code is still pending (backlog, section 13).
20. Corpus runner: the `doorplate::corpus` module (feature `corpus`), used by `tests/corpus.rs` and `doorplate corpus test`. Exit codes: 0 ok, 2 malformed corpus, 3 regression or stale baseline.
21. `doorplate` CLI (alias `addr`) (section 8). The CLI, not the core, depends on `csv` and `serde_json`.
22. `benches/parse.rs`. First numbers (quick run, 2026-10-15): about 4 to 11 µs per address, about 110K addresses/s one by one, about 3x that through `parse_many` with `rayon`. Run `cargo bench -p doorplate --bench parse` (add `--features rayon`).

Also done ahead of its milestone: the usaddress comparison script and
results (`tools/compare/compare_usaddress.py`,
`docs/usaddress-comparison.md`), first planned for Milestone 8.

### Milestone 5: place list (done)

The first state-pack work (section 11), scoped to what the parser needs to
find the city. It ended with the benchmark rerun that made the CRF call: no
CRF (decisions 10 and 32). What each item built, with the evidence in
`CHANGELOG.md`, `docs/places.md` and `docs/packs/fl.md`:

- **Florida pack** (`data/packs/fl/`, feature `state-fl`), built by
  `tools/packs/build_fl.py` from auction-tracker's `cities_fl.json`, plus
  observed ZIP places from the DBPR export and the `CR` override; decisions 13
  to 21. Florida lookups behind the `Lexicons` interface: the city on one line
  with no commas, a city run into the street, street versus city before a ZIP,
  cut-off cities completed, the place from the ZIP, `zip_city_mismatch`.
  Corpus `fl-001` to `fl-033`.
23. **Slug follows the street for dual addresses** (decision 11). `rg-040` to
    `rg-044` changed.
24. **National place data** (`data/places/`, feature `places-us`):
    `tools/places/build_us.py` builds 50,375 places and 41,154 ZIPs for the 50
    states, DC and Puerto Rico from the Census Gazetteer and GeoNames;
    `build.rs` compiles them under the packs (decision 29) as packed binary
    tables (+2.9 MB in the binary; 13.4 MB as `&str` tables, which is why they
    are packed); GeoNames attribution in `NOTICE`.
25. **The national list in the parser:** place lookups read all 52 states, and
    `zip_city_mismatch` stays limited to states with a curated pack (decision
    30). Corpus `pl-001` to `pl-008`. Blind geocode benchmark, whole address
    (coarse): 85.7% to 96.1% (usaddress 88.2%); parse time +6.5% on the
    corpus.
26. **Florida `SR` and county roads:** `[[highway]]` entries in the pack's
    `overrides.toml` read through `Lexicons::highway_in` (`SR` is STATE ROAD,
    `CTY HWY` is COUNTY HIGHWAY). Corpus `fl-020` to `fl-023`.
27. **Rarer misspellings and the penthouse shape:** `DRT`, `LNE`, `SU` as
    observed spellings; `APT PH 5213` is `APT PH5213` (decision 31). Corpus
    `rg-098` to `rg-105`.
28. **Florida communities still missing:** hand-kept `places_added.tsv` and
    `variants_added.tsv` beside the pack, chosen with
    `tools/packs/measure_fl_places.py` (at least 10 distinct addresses). Two
    rounds against the DBPR exports: 17 + 19 communities, 21 + 16 spellings,
    341 observed ZIP pairs; the scripts read every DBPR export together
    (`tools/packs/dbpr.py`).
29. **Rerun the comparisons and make the CRF call:** decision 32, no CRF.
    Blind benchmark 96.1% against 88.2%; usaddress's own files 99.2% against
    its 100.0% (its regression tests). Items 39 to 42 and 52 dropped.

Also done in this milestone, from three more DBPR licensee files (613,719
distinct addresses): four parser fixes (a cut-off copy ahead of the address,
designators joined to values, a name on line 1, BAY and STUDIO units), corpus
`rg-106` to `rg-120`.

### Milestone 6: usaddress-shaped features and the Python wrapper (done)

30. **`ParseOptions::default_state`** (decision 24): `Parser::with_options`
    (`OptionsError` for a non-state), `--default-state`, warning
    `default_state_applied`, a `default_state` field on corpus cases. Used for
    the tagger's place context (a comma-less city), `place`, and the
    state-dependent suffix and highway readings; only when the input names no
    state and has no known ZIP. Corpus `ds-001` to `ds-007`; on state-less
    Florida auction rows a place is found for 8,828 of 9,314 (1,273 without).
31. **`tag()` and `address_type()`** (decision 23): `ParsedAddress::tag()`
    (`TaggedAddress`), `address_type()`, `Warning::RepeatedLabel` (reported by
    `tag()`, never in `Diagnostics`), `doorplate parse --tag`, corpus fields
    `address_type`, `tag`, `tag_repeated` (`tg-001` to `tg-008`), and
    `compare_usaddress.py tag`: the joined dicts are identical to
    `usaddress.tag()` on 99.3% of 8,103 addresses, the types on 99.8%.
32. **Migration notes:** the README's "Coming from usaddress" and the Python
    README's table.
33. **The wrapper:** `crates/doorplate-py` in the workspace (`test = false`, so
    plain `cargo test` and `clippy` need no libpython), the package, stubs and
    `py.typed`, 27 pytest tests, a CI job. Section 7.
34. **`parse_many`:** releases the GIL and parses in parallel; `test_batch.py`,
    `benchmarks/bench_batch.py` (100,000 addresses: 1.55 s for a loop of
    `parse`, 0.25 s for `parse_many`, 6.3x), and tested pandas and Polars
    recipes. A columnar call (or Arrow) could remove the cost of reading
    attributes from Python objects; not built.
35. **Wheels:** `wheels.yml`, one abi3 wheel each for Linux x86_64 and aarch64
    (manylinux), macOS universal2 and Windows x64, installed and tested on
    Python 3.10 and 3.13 on every pull request. Nothing is published.

### Milestone 7: calibrated confidence (closed; item 38 deferred)

Runs before the rest of Milestone 8 (items 44 to 46; decision 41, which
replaces the order of decision 38): the Florida sample can find parser bugs
that change keys, and those must land before `k1` freezes at the first
release. Items 37 and 36 are done. Item 38 was started and then deferred
past the first release (decision 43); the first release ships
`confidence = None`.

Calibrated confidence (decision 12):

36. **Blind Florida sample** **(author)**: hand-label 100 to 200 rows of the
    private Florida files, stored in `local-data/` with the labeling
    conventions of `tests/corpus/README.md`. It is calibration data and a
    blind test on the data this library targets. The rows never leave
    `local-data/`; only numbers fitted from them ship.
    - Commit 1 (done): `tools/calibrate/sample_fl.py` draws 150 rows
      uniformly (seeded) from distinct auction and DBPR addresses, plus 40
      from rows the parser flags, skipping corpus inputs, and writes them as
      corpus records pre-filled with the current parse (`TODO` in the note
      until checked). Drawn 2026-10-17 into `local-data/florida-sample/`
      (190 records), with a labeling guide there; scored by
      `doorplate corpus test --dir local-data/florida-sample`.
    - Commit 2 (done): `tools/calibrate/label.py`, a terminal tool for the
      labeling: the draft hidden until asked for, standard values picked
      from `data/*.toml`, the field-driven warnings computed, each row
      checked by the corpus runner before it is saved, and `NOTES.md`
      written from the differences with the parser.
    - Commit 3 (done): `tools/calibrate/triage.py` and the collaborative
      mode in `tools/calibrate/README.md`: an independent check sorts the
      drafts into clear and review, the assistant accepts the clear rows it
      would label the same way and proposes labels for the rest, and the
      author approves or overrules each. Who labeled a row is in its note.
    - Labeled 2026-10-17 (190 rows; who labeled each is in its note). Whole
      address 178 of 190 (146 of the random 150) before fixes.
    - Commit 4 (done): seven parser fixes from the sample's errors, corpus
      `rg-121` to `rg-136`, key changes logged in `docs/key-format.md`; the
      sample then scores 185 of 190 (149 of 150). Two fixable shapes stay in
      the backlog; three rows are bad source data.
37. **Record the tagger's uncertain decisions (done).** `tagger::Decision`
    in `Tagged::decisions`, each once in the order made: `CityByComma`,
    `CityBySuffix`, `CityByPlace`, `StateWithoutZip`, `BareUnit` (after the
    suffix, or a line that is only a number), `NumberLetter` and
    `DefaultState` (a city or a pack highway spelling found under the hint).
    Internal (`pub(crate)`, read by item 38), with unit tests in
    `tagger/mod.rs`; no output or key changes (the corpus is unchanged).
38. **Fit and ship the score (deferred past the first release, decision 43).**
    - Commit 1 (done, kept as groundwork): `tools/calibrate/fit.py` fits a
      small logistic model on the golden corpus, usaddress's test files and
      the Florida sample (9,633 examples, only 52 wrong parses). The sample
      was used to fix seven rules (item 36, commit 4), so it is scored with a
      parser built before those fixes; the geocode benchmark is kept for
      checking. The tagger's decisions reach `Diagnostics` (internal) and the
      tools read them from `doorplate corpus features` and `doorplate parse --features`.
      No parse or output change.
    - Result, which is why the rest is deferred: the unconstrained fit was
      unstable and misleading (it rewarded `missing_state`, a feature of the
      synthetic test files); constrained so that no warning raises
      confidence, it keeps six features and gives about 0.98 to most parses,
      about 0.87 to a missing street number or a bare unit and about 0.58 to
      an unparsed remainder. That separates clean parses from a few suspect
      kinds and nothing more, and the Florida sample alone (190 rows, 12
      failures) gives intervals too wide to claim more. The report is
      `local-data/reports/confidence-fit.md` (private, not committed).
    - Not started: the fitted table in `data/` compiled by `build.rs`, and
      `docs/confidence.md`. What a later attempt needs is the backlog entry
      "Calibrated confidence" in section 13.

**No CRF fallback.** Item 29 decided no-go (decision 32), so the CRF items 39
to 42 (a `crfs` spike, `tools/train/`, `models/`, `src/tagger/crf.rs`) are
dropped. The numbers are kept so that references stay stable. User
customization is the custom layer (section 11; deferred, decision 45), which needs no
Python toolchain and no retraining.

### Milestone 8: hardening and release

In this order. The parser changes (53 to 56) came first so that fuzzing and
the Pub 28 recheck cover the final rules. Items 44 to 46 followed Milestone 7
(decision 41): the Pub 28 recheck covers the rules the Florida sample changed.
Item 38's fitted table is not packaged (decision 43).
Items 57, 58, 61 and 62 (decision 42) prepare the release: measured
benchmarks, a name, the text cleaning and a fresh repository. Release order:
44, 57, 58, 45, 61, 62, 46 (preceded by the Milestone 9 items of decision 44).

53. **Remove the unused surface (done)** (decision 35): `Diagnostics::model_version`
    (Rust, Python property, stub) and the empty `crf` cargo feature. No parse
    changes.
54. **Trailing country (done)** (decision 36): recognize `USA`, `US`, `UNITED
    STATES` (and `U.S.A.`) after the ZIP or state, drop it from the address
    and raise a new warning `country_ignored` (`Warning::ALL` grows by one).
    Other countries stay out of scope. Corpus cases for each shape,
    including a country in the city field or on its own line.
55. **Intersections (done)** (decision 37): detect `MAIN ST & 1ST AVE` (and `AND`,
    `@`, `AT` where unambiguous) and raise a new warning
    `intersection_detected`; the second street stays in `unparsed` and the
    key stays `None`. No new fields. Corpus cases, including street names
    that contain `AND` (`LAND AND SEA RD`) and must not warn.
56. **Netsyms comparison (done)** (decision 40): `tools/netsyms/` (sample,
    profile, compare), eight parser fixes found by scoring 217,906 uniform US
    rows (99.8% whole-address on rows without source quirks, up from 99.2% to
    99.5%), corpus `ns-001` to `ns-023` (invented addresses), results in
    `docs/future-sources.md`. The geocode benchmark moved from 96.1% to 96.4%.
43. **Fuzz targets (done)** (cargo-fuzz) for the tokenizer and assembler, reached through `Parser::parse` and `Parser::parse_fields` plus every emitter. `fuzz/` is outside the workspace (nightly only). Run: `cargo +nightly fuzz run parse` from `fuzz/`.
44. **Re-check every lexicon table and the USPS emitter rules against the
    live Pub 28 pages (done 2026-10-17)** (`docs/usps-pub28-notes.md`,
    "Verification status"). Last before release, so the check covers the final
    tables. `tools/pub28/verify.py` diffs the C1, C2 and B tables against
    `data/` (0 differences) and, with `--emitter`, runs every Appendix F
    example through the parser. Two bugs found and fixed: the first of two
    suffix words is spelled out (section 234.2) and a highway route keeps up
    to two letters (`ROUTE 1150EE`); 958 of 440,288 Florida licensee keys
    changed (`docs/key-format.md`), the geocode benchmark is unchanged at
    96.4%.
57. **Benchmarks for the release** (decision 42): measure and write up, in
    `docs/benchmarks.md` (which replaces `docs/usaddress-comparison.md` in
    item 61): accuracy on the blind geocode benchmark, the golden corpus and
    the hand-labeled sample; speed in Rust (one by one, `parse_many` with
    `rayon`) and in Python (`parse`, `parse_many`, against usaddress on the
    same inputs and machine); memory, binary and wheel size. Confidence is not
    measured: it is `None` (decision 43). Scripts in `benchmarks/` so the numbers can be
    rerun; machine and versions recorded. usaddress is one reference point,
    not the frame.
58. **Choose the name (done: `doorplate`, decision 46)** **(author)** (decision 42, replacing the names of
    decision 33): one name for the Rust crates, the Python package, the
    import, the CLI and the repository, free on crates.io, PyPI and GitHub
    and not easily confused with an existing crate or package (the current
    `us-address-parser` collides with the `us_address_parser` crate). Research
    the candidates (registries, search, trademarks in the address-data
    space) in `docs/naming.md`, with a short list for the author; the author
    picks. Output: the names and a rename map (every crate, package, module,
    binary, path, URL, doc and example) that item 45 applies.
45. **Rename and package for publishing** (names from item 58; the rename
    is done, the packaging is not):
    - **(done)** Apply item 58's rename map in one pass: crates, the Python package and
      import, the CLI binary if renamed, paths, CI, docs and examples; then
      `git grep` for every old name (decision 33's names are replaced).
    - Put the data inside the packages: copy `data/` into the `doorplate`
      crate (`build.rs` already looks for a crate-local `data/` first) so
      `cargo package` succeeds and `maturin sdist` carries the tables;
      keep one source of truth (a script or a check that the copies match).
    - Check the names again just before publishing.
61. **Text cleaning** (decision 42): apply `docs/repository_cleaning.md` in
    full: build-process language out of the docs and comments (`design.md`
    becomes a historical outline), the private data sources described
    generically with every required attribution kept, the README rewritten
    for users with diagrams of how the parser works, a neutral tone toward
    usaddress around item 57's benchmarks, no old names. Text only; one
    pull request per section of that document.
62. **Fresh repository** (decision 42): copy the cleaned tree into a new
    repository under the new name as one initial commit, with no history,
    after the private-data scan in `docs/repository_cleaning.md` section 6;
    CI and publishing set up there; the old repository archived.
46. **Publishing**, from the fresh repository: crates.io and PyPI workflows
    (the wheels already build); the CHANGELOG's single first-release entry;
    from then on a change to what an input produces needs `k2` (decision
    34).

### Milestone 9: customization and extras (items 50 and 59 ship in the first release and come first; items 47 to 49 and 60 are deferred, decision 45; item 51 is not in it either, decision 44)

Features that make the library adaptable to a user's own data. None of them
changes the shipped data or the `k1` key format.

47. **Deferred (decision 45).** **Custom layer, format and loader** (section 11, "Custom layer"; decision
    22): `CustomLayer` in `doorplate` (feature `custom-layer`), built from
    TOML text with the same table shapes as a state pack (`pack.toml`,
    `places.tsv`, `zips.tsv`, `overrides.toml`) plus `aliases.toml` for
    suffix, unit, directional and city spellings. Validation errors name the
    file, line and rule; add-only conflicts are errors. Applied through the
    `Lexicons` interface, so the tagger gains no layer-specific logic.
    Scope (decision 42): as broad as data can reach. Before building, list
    every table and word list the tagger reads through `Lexicons` (unit
    words, placeholders, building words, highway prefixes, special words,
    state-scoped readings) and make each one extendable by a layer unless
    there is a reason not to, recorded in section 11.
48. **Deferred (decision 45).** **Layer provenance in output:** `Diagnostics::custom_layer` (name,
    version, content fingerprint) records which layer produced a parse; no
    per-parse warning is added. Documented in `docs/key-format.md`: the key
    depends on the layer in use, so users who store keys must keep the layer
    fixed or re-key.
49. **Deferred (decision 45).** **CLI and Python access:** `doorplate --custom-layer PATH`, `doorplate layer check
    PATH`, `Parser(custom_layer=...)` and a test suite that runs the golden
    corpus with a sample layer (`tests/corpus/layers/`).
59. **Examples (done)** (decision 42; no layer example, decision 45): an `examples/` directory with invented
    addresses only: sample CSV and text input files (one line, split fields,
    messy real-world shapes), a Jupyter notebook (`parse`, `parse_many`,
    `parse_fields`, keys and slugs, the CLI on a file), and
    Polars and pandas pipelines (over item 50's helpers once they exist).
    Run in CI (the notebook executed with `nbclient` inside
    `crates/doorplate-py/tests/test_examples.py`) so it
    stays current. Lands before item 61, which links to it.
60. **Deferred with items 47 to 49 (decision 45).** **Customization guide** (decision 42): one place, `custom/` at the
    repository root (or `docs/customizing.md` with `custom/template/`),
    that a new developer, or a capable LLM given the guide, can follow to
    build a layer without reading the code: what each file changes, a
    commented template layer, worked examples (a local community, a street
    spelling, a state-scoped reading), the checks to run (`doorplate layer
    check`, the corpus with the layer), and a ready prompt for an LLM.
    After items 47 to 49.
50. **Dataframe helpers (done; optional extras):** `doorplate.pandas.parse_column`
    and `doorplate.polars.parse_column` return component columns, key,
    slug, address type and warnings (`confidence` is `None`, decision 43, so
    it is left out); implemented over `parse_many` through one shared column
    reader (`_columns.py`); the library is imported when the function is
    called; tests skip when it is absent; extras `[pandas]` and `[polars]`.
    Decision 26. The Polars expression namespace was not built (it would parse
    row by row or copy the column twice; the function over a Series is as
    cheap); reopen only if asked. Output is all strings (`string` dtype in
    pandas, `String` in Polars), null for a row with no address text.
51. **Optional emitters, if wanted** (the display-form and long-form open
    decisions in section 13): a mixed-case display form and a long-form
    (unabbreviated) form, one pull request each. Not in the first release
    (decision 44); stays an option for a later version.
52. **User model file: dropped** (decision 32). There is no CRF to load a
    model into. Item number kept.

---

## 11. Deferred feature: state tuning packs

Built: the national list and the Florida pack (Milestone 5). Waiting: other
states' packs, which are added where real data shows a need, and the custom
layer (Milestone 9). This section records the design so the core stays ready
for them.

**Idea:** the base parser is US-wide and state-agnostic. Data comes in
layers, each a set of files compiled in by `build.rs`:

1. **Base lexicons** (`data/*.toml`): Pub 28 and RESO tables.
2. **National place list** (`data/places/`, feature `places-us`, on by
   default): generated for every state, DC and Puerto Rico from public
   data (Census Gazetteer, GeoNames ZIPs). Good enough to find cities and
   match ZIPs everywhere; no misspellings.
3. **Curated state packs** (`data/packs/<st>/`, feature `state-<st>`):
   hand-checked data for a state, added where real data shows a need.
   Florida is the first (on by default).

```
data/packs/fl/
  pack.toml        # state code, pack version, sources and licenses
  places.tsv       # canonical place names, county, kind, variants (abbreviation or typo)
  zips.tsv         # ZIP -> main place, shared places
  zips_observed.tsv # more (ZIP, place) pairs seen in real data
  places_added.tsv  # hand-kept communities seen in real data, with address counts
  variants_added.tsv # hand-kept spellings of known places, with address counts
  overrides.toml   # suffix spellings read differently in the state (CR) and highway prefixes (SR)
```

Layering is add-only: a later layer adds names, variants and ZIPs and
never replaces or removes an earlier entry; a conflict fails the build.
Where a state has a curated pack, the pack wins and national rows for that
state only add places it lacks (section 13, decision 29).
Packs are fixed at compile time (section 3, decision 7); there are no user
packs loaded from a directory. State selection is two-pass: find the
state (state token or ZIP prefix), then look up places in that state.

**Work waiting on packs** (from the corpus, `docs/prior-art.md` and
`docs/usaddress-comparison.md`), with where each is tracked:

- A place list to find the city on one line with no commas (`ST LOUIS`,
  `MOUNTAIN VIEW`, `W. PALM BEACH`). *Done everywhere*, along with a city
  run into the street (`13551 WILSON STREETDADE CITY`), cut-off cities
  completed from the ZIP (`WESLEY CHA`), the place from the ZIP, and
  `WINGATE ST 32507` (street plus ZIP, not city plus ZIP). The national list
  finds 87.4% of the blind benchmark's cities (items 24 and 25).
- `SR` (State Road in Florida, State Route elsewhere), `RR` as Ranch Road in
  Texas, `CR` as CIRCLE in Florida (*done*: the Florida pack's
  `overrides.toml`, decision 20), `PR 462` (Texas private road vs Puerto
  Rico), `CTY ROAD`, `LEE ROAD 2004`-style county road names. Florida's
  `SR` and `CTY HWY` are done (item 26); the rest are in the backlog.
- Queens-style hyphenated house numbers, Puerto Rico urbanizations
  (backlog).
- `--default-state` for `doorplate parse-file` (most of the 1,700 auction rows
  with no state). *Done* (item 30, decision 24): `ParseOptions::default_state`
  and `--default-state`.
- `ST`/`FT`/`MT` vs `SAINT`/`FORT`/`MOUNT` in place names, and suffix
  abbreviations inside place names (`ALTAMONTE SPGS`). *Done*: the place
  comparison rule (`lexicon/place_key.rs`).

Settled 2026-10-16 (section 13, decisions 13 to 19): add-only layering,
compile-time packs, the pack version recorded in `pack.toml` but not yet in
output, and GeoNames (CC BY 4.0) as the ZIP source. Packs carry no model
(decision 32). Still open, in the backlog (section 13):

- A required golden-test corpus per pack, enforced in CI (`doorplate pack test`).
  The custom layer reuses it: `doorplate corpus test --custom-layer`.
- Reporting the pack version in output. For the custom layer the fingerprint
  is reported (Milestone 9, item 48); shipped packs may follow.

### Custom layer (user customization; deferred past the first release, decision 45)

The same idea as a state pack, owned by the user and loaded at run time.
Settled in decision 22; not built until a real layer needs it (decision 45).
The design:

**What it is.** A directory (or in-memory text) in the state-pack format:
`layer.toml` (name, version, notes; same fields as `pack.toml`), and any of
`places.tsv`, `zips.tsv`, `overrides.toml`, plus `aliases.toml`. It is a
fourth lexicon layer, above the state packs (section 11 list), reached only
through the `Lexicons` interface.

**What it can hold** (each entry needs an `evidence` or `note` field, as
`overrides.toml` does):

| File | Content | Example |
|---|---|---|
| `aliases.toml` | Extra spellings of a street suffix, unit type, directional or state name | `[suffix] "PKY" = "PKWY"`; `[unit] "SUITE#" = "STE"` |
| `places.tsv` | Extra places and variant spellings (abbreviation or typo) | a local community the national list lacks |
| `zips.tsv` | ZIP to place, shared places | `33101 = MIAMI` |
| `overrides.toml` | Read a base suffix as another in one state (as the Florida pack's `CR`) | `[ [override] state = "TX", spelling = "RR", suffix = "RANCH ROAD" ]` |
| `layer.toml` | Optional `default_state` and a named list of `treat_as_unit_words` or `ignore_words` | `ignore_words = ["USA"]` |

**Rules.**
- *Add-only* (decision 18): a layer adds names, variants, aliases and ZIPs
  and never replaces or removes an entry of the base or a state pack. A
  conflict is an error at load, naming the entry. The single exception is the
  state-scoped `overrides.toml` reading, as decision 20 already allows.
- *Deterministic and pure:* a layer is data, not code. The same layer and
  input give the same output on every platform.
- *Validated at load:* unknown keys, bad state codes, duplicate or
  conflicting entries and malformed rows are errors, never panics and never
  silently ignored. A bad layer fails to load; parsing never starts with a
  half-loaded one.
- *Tagger unchanged:* no layer-specific logic in the tagger (CLAUDE.md rule).
  Lookups go through `Lexicons`, which merges base, pack and layer.
- *Provenance:* `Diagnostics::custom_layer` carries the layer's name,
  version and content fingerprint (Milestone 9, item 48).
- *I/O outside the core:* `doorplate` takes layer text; the CLI and the
  Python wrapper read files.

**Effect on the key.** A layer can change how a string is read (an alias moves
a token from street name to suffix), so it can change the `k1` key of an
input. The key format does not change (no `k2`); the key is a function of the
parse, which is a function of the layer. Users who store keys keep the layer
fixed or re-key; the fingerprint says which layer produced a stored key.
`docs/key-format.md` gets a short note when item 48 lands.

**What it does not do.** It does not change the grammar (new address shapes
need code), the output formats, or the key format, and it does not validate
or correct addresses. Rare shapes it cannot express (intersections, landmarks)
are the open decisions in section 13.

**No CRF.** The layer is the customization path, and the only one: there is
no trained model to retrain (decision 32). It works with the rule tagger and
feeds the rule stage and the place lookups.

**Scope and guide (decision 42).** The layer should reach every table and
word list the tagger reads through `Lexicons`, not only the five files above;
item 47 starts by listing them and records here any it leaves out and why.
Building a layer must be approachable for a developer new to the code, or for
a capable LLM working from instructions: one central guide with a commented
template, worked examples and the checks to run (item 60).

**Design-now constraints:** keep all lexicon access behind the `Lexicons`
interface; keep the key independent of city (section 6); keep diagnostics
extensible; no state-specific rule in the tagger.

---

## 12. Risks

- **Accuracy parity with usaddress.** On the blind geocode benchmark (30,622
  one-line addresses) this parser gets 96.4% of whole addresses right
  (coarse) against usaddress's 88.2% (96.1% before the fixes from the Netsyms
  comparison, decision 40; 85.7% before the national place list). On usaddress's own test files it gets 99.2% (those are
  usaddress's regression tests, so usaddress scores 100% by construction).
  The benchmark has no license and the rules were not tuned on it, but they
  were checked against it repeatedly; the hand-labeled Florida sample (item
  36) is the cleaner blind test. Details: `docs/usaddress-comparison.md`.
- **Key stability.** `k1` is the public format, frozen at the first release
  (decision 34). Until then a parser fix can change the key an input
  produces; each such change is logged in `docs/key-format.md`.
- **Licensing.** Keep MIT attribution for vendored usaddress test data and
  any usaddress-derived training data. Unlicensed test data (the geocode
  benchmark) stays in `local-data/` and is never committed.
- **Fuzzing** (item 43) ran for minutes per target before the first release, not
  hours; run longer before publishing (item 46). Proptest also covers the
  never-panic rule.
- **Python boundary overhead** for single-address calls (about 15 us each);
  `parse_many` is 6x faster on a batch, and the README says so.
- **Packaging.** An sdist and a crate lack `data/` until item 45; the
  crates.io name `doorplate` is taken (decision 33). Both are known
  and scheduled.
- **Migration friction from usaddress.** Users expect `tag()`, the label names
  and an `address_type`, and a way to adapt the parser to odd data. The first
  three are done (Milestone 6); the last is the custom layer (deferred,
  decision 45).
- **Custom layer and key stability.** A layer can change a parse and so a
  key. Mitigation: add-only layers, a fingerprint in diagnostics, and a
  documented rule that stored keys need a fixed layer (section 11).

---

## 13. Decisions and open items

### Decisions recorded 2026-10-15

1. **Florida parsing code: pending.** The author will provide it later, as a reference (not ported). It seeds the corpus when it arrives.
2. **Names: `us-address-parser` (PyPI only; see decision 33).** The Python package is `us-address-parser` (import `us_address_parser`); the CLI binary stays `addr`. The crates.io names changed in decision 33 because `us-address-parser` is taken there. The rename applies at Milestone 8 (item 45); until then the crates keep the `addr-core`, `addr-cli` and `addr-py` names. **Replaced by decision 42:** all of these names are dropped for one name chosen in item 58.
3. **Licensing: confirmed.** MIT OR Apache-2.0.
4. **Key definition: signed off.** `k1` (section 6) is the public format; from the first release any change to what an input produces needs a new version prefix (decision 34 sets when it freezes). **Minimum Python version: 3.10** (abi3 wheels).
5. **RESO mapping: reviewed.** All choices accepted as implemented; see "Decisions" in `docs/reso-mapping.md`.
6. **Dual addresses (Pub 28 section 215): hold both.** A record with a street address and a PO Box keeps both (street fields and `po_box`, warning `dual_address`). The key and the RESO output use the street; the USPS output writes the PO Box alone, as section 215 prefers. Rejected: keeping one and leaving the other unparsed, and returning two addresses.
7. **Attention lines: a field.** `ParsedAddress::attention` holds an ATTN or C/O line (section 214). No emitter writes it.
8. **Rule changes from the usaddress comparison and geocode benchmark** (`docs/usps-pub28-notes.md`, rows added 2026-10-15): a no-ZIP state at the end of one line (never NE or FL; suffix-like codes only after an abbreviated suffix); hyphenated routes split (`GA-25`), SR left to state packs; `BOX <number>` alone is a PO Box; a letter before a directional joins the house number; letter-and-digits house numbers (`T703`); two-part grid numbers kept as written; letter routes after a county prefix; `COUNTRY ROAD` before a number is an observed typo; BUSINESS after a route stays in the name; `missing_street_name` for a number and unit with no street; ordinal floors normalized to numbers (`FL 5`); `STE 4 551` stays unparsed; a `building` field.
9. **Deferred to state packs:** the items listed in section 11.

### Decisions recorded 2026-10-16

10. **CRF fallback: decided after the place list.** Milestone 5 builds the
    place list and reruns the blind benchmark (item 29); the CRF items of
    Milestone 7 go ahead only if a gap remains that rules plus the place
    list are unlikely to close. **Decided no-go, 2026-10-16: decision 32.**
11. **Slug for dual addresses: follow the street.** The key and the RESO
    output already use the street; only the USPS output writes the PO Box,
    because Pub 28 section 215 says so for the mailpiece. The slug is
    built from the street line, as the RESO emitter's `UnparsedAddress` is
    (the shared `street_line()` helper in `emit/usps.rs`), so every
    identifier names the same place. The slug is not a contract, so this
    needs no version change. Done (item 23).
12. **`Diagnostics::confidence`: a calibrated probability.** A number from
    0.0 to 1.0: the chance that the whole parse (every component) is right,
    calibrated so that of the addresses scored 0.9, about 90% are right on a
    labeled set the rules were not tuned on. It stays `None` until it is
    calibrated (Milestone 7, items 36 to 38). Plan:
    - The rule tagger has no native probability, so the score comes from
      the decisions it had to make: which warnings fired, and which uncertain
      rules settled a boundary (city found by a comma, by the no-comma
      suffix split or by the place list; a state with no ZIP; a bare unit; a
      house-number letter rule). The tagger records these internally.
    - The accuracy of each path is measured on labeled data and compiled
      into a fixed table (or a small logistic model) in `data/`, with its
      source, so the score stays deterministic and has no runtime
      dependencies.
    - Fit on licensed data only: the golden corpus, usaddress's MIT test
      files, and the hand-labeled Florida sample. The geocode benchmark has
      no license, so it is used to check calibration, not to fit numbers
      that ship.
    - Calibration waits for the place list, because the place list changes
      which paths are uncertain.
    - **Deferred past the first release (decision 43).** The plan above
      stands as the target; the data to meet it does not exist yet.

### Decisions recorded 2026-10-16 (place list)

13. **A place found from the list is reported beside the city, not in
    it.** `ParsedAddress::city` stays what the input said (no span when it
    came from the ZIP). A new `place` field holds the standard place name
    and how it was found: exact, variant, completed (cut off) or from the
    ZIP. Emitters keep writing the city as written. Filling or correcting
    a city is a correction, which this library otherwise avoids; the key
    leaves the city out, so none of this changes `k1`.
14. **City and ZIP disagreeing is a warning** (`ZipCityMismatch`). It says
    the two do not match the place list; it does not say the address is
    undeliverable.
15. **Florida first, then a national base list.** The Florida pack proves
    the format and the `Lexicons` methods on curated data; the national
    base (Census Gazetteer plus GeoNames) follows in the same format. The
    blind benchmark is rerun after both. The other 49 states get generated
    data, not hand-built packs; curated packs are added later where real
    data shows a need.
16. **Features on by default.** `places-us` and `state-fl`; the Python
    wheel includes both. The Florida data is under 100 KB; the national
    list is expected at about 1.5 to 2 MB compiled.
17. **ZIP data: GeoNames, CC BY 4.0.** The Florida ZIP table in
    `cities_fl.json` is GeoNames data, and the GeoNames US postal file is
    the national source. It ships with attribution in `docs/packs/` and a
    NOTICE file; the code stays MIT OR Apache-2.0. Rejected: Census ZCTAs
    (public domain, but not USPS ZIPs and no PO Box ZIPs) and the USPS City
    State file (no redistribution).
18. **Pack questions settled:** layering is add-only, packs are fixed at
    compile time (no user packs from a directory), and the pack version is
    recorded in `pack.toml` but not added to output yet.
19. **Florida variants come from auction-tracker's `cities_fl.json`
    only.** An older copy of the list elsewhere is redundant. Florida road
    names (`SR`, `CR`; the later `overrides.toml`) stay out of this pass.

### Decisions recorded 2026-10-16 (DBPR data)

20. **Packs may read a suffix spelling differently.** A pack's
    `overrides.toml` maps a spelling the base suffix table already has to
    another suffix in that state, so tagging never changes, only the value
    (resolved in `assemble` through `Lexicons::suffix_in`, from the
    address's state or its ZIP's). This is the one place a pack replaces a
    base reading rather than adding to it (decision 18); each entry names
    its evidence. First entry: `CR` is CIRCLE in Florida (`docs/packs/fl.md`),
    asked for by the author on the DBPR evidence. This narrows decision 19
    for `CR` as a suffix; `SR` and county road names are still open.
21. **Observed ZIP places from DBPR data.** The Florida DBPR licensee export
    (public record) is an approved source for pack data and corpus cases.
    City and ZIP pairs seen often enough there are added to ZIPs' shared
    places (`zips_observed.tsv`, `tools/packs/observe_fl_zips.py`);
    `zip_city_mismatch` on that data fell from 5.4% to 0.4% of rows.

### Decisions recorded 2026-10-16 (usaddress feature review)

From the feature comparison in `docs/usaddress-comparison.md`. Decisions 22
to 24 and 26 are drafted from the author's stated intent and recommendations;
revisit if they cause friction.

22. **Customization is a runtime custom layer in the state-pack format, not a
    model.** Users add aliases, places, ZIPs and state-scoped suffix readings
    in a layer loaded at run time (section 11, "Custom layer"; Milestone 9).
    It is add-only, deterministic and validated at load, and goes through the
    `Lexicons` interface. This narrows decisions 7 and 18 ("no user packs from
    a directory"): shipped data stays fixed at compile time, but the user layer
    is loaded at run time. Rejected as the primary mechanism: retraining the
    CRF (needs the Python toolchain and labeled data, and moves other parses;
    since dropped, decision 32), and an editable copy of the shipped tables.
23. **`tag()` and `address_type()` are core emitters.** `tag()` joins tokens
    by usaddress label, `address_type()` classifies the address (finer than
    usaddress's three values and mapped to them for migration). Neither fails
    on a repeated label (warning `repeated_label`). Python exposes both
    (Milestone 6, item 31).
24. **`default_state` is a lookup hint, never a fill.** When the input has no
    state and no ZIP, the place list uses the hint to find the city; `state`
    stays empty, `missing_state` still fires and the key is unchanged
    (consistent with decision 13). A warning `default_state_applied` fires
    when the hint decided a place match. Available as a parse option, a CLI
    flag and a layer setting.
25. **Out-of-scope usaddress labels are documented, not hidden.** The README
    and the migration notes list the labels this library does not model
    (the open decisions below decide which, if any, to add).
26. **Dataframe helpers are welcome, later, as optional extras.** The core
    and the base wheel stay dependency-free; `[pandas]` and `[polars]` extras
    wrap `parse_many` (Milestone 9, item 50). The README recipe comes first.

### Decisions recorded 2026-10-16 (planning review)

27. **Python before calibrated confidence.** Milestone 5 used to end with
    calibration so the wrapper would ship with it. Nothing is published
    before Milestone 8, and `confidence` is already in the API (`None`
    until calibrated), so the wrapper does not depend on it; calibration
    does depend on the hand-labeled Florida sample (item 36), which is the
    author's work. Calibration moves to Milestone 7 (the CRF items that were to follow it
    are dropped, decision 32). It still lands before the first release.
    *Superseded by decision 43: it is deferred past the first release.*
28. **One pull request per plan item.** Section 10 lists the remaining work
    in build order as numbered items, each one pull request; commits within
    an item are listed where there are several. Open items found while
    building go into section 10 if they are scheduled, otherwise into the
    backlog below. `docs/packs/<st>.md` keeps the evidence for its state's
    items.
29. **A curated pack wins over the national list for its state.** The
    national data has its own row for every Florida place, sometimes with
    another spelling (the Census's `Everglades` is the pack's `EVERGLADES
    CITY`) or another main ZIP. Where a state has a curated pack, the
    pack's places and ZIPs are used, and the national rows for that state
    only add places the pack lacks, matched by the place comparison rule.
    Add-only layering (decision 18) still holds. Item 24.
30. **`zip_city_mismatch` only where a curated pack covers the state.**
    GeoNames gives one place per ZIP; on that alone Florida fired on 5.4%
    of DBPR rows before the observed ZIP places (decision 21), so the
    national list would make the warning noisy. It fires only for states
    with a curated pack, and its docs say so. Item 25.
31. **A penthouse after a designator is one unit.** `APT PH 5213`, `UNIT
    PH6` and `SUITE PH` (162 DBPR rows) give a designator with no number
    plus a PH unit today. They become one unit: the designator with `PH`
    joined to the number (`APT PH5213`, `UNIT PH6`; `SUITE PH` is `STE
    PH`). Item 27 (done).

### Decisions recorded 2026-10-16 (CRF call)

32. **No CRF fallback.** Item 29 reran the usaddress comparison and the blind
    geocode benchmark after the national place list and the Florida work
    (`docs/usaddress-comparison.md`). Decision 10 asked whether a gap
    remains that rules plus the place list are unlikely to close. None does:
    - On the blind benchmark (30,622 addresses neither parser trained on),
      whole address (coarse) is 96.1% against usaddress's 88.2%, and
      `PlaceName` tokens tie at 96.8%. The place list moved it from 85.7%.
    - usaddress still wins only on its own test files (ours 99.2% against its
      100.0%), which are its regression tests and training data, so they
      measure closeness to usaddress and not accuracy.
    - The largest remaining error (1,161 `PlaceName` tokens taken for
      `StreetName`, such as `281 EVELYN DR DOTHAN`) is a row with no state
      and no ZIP. A CRF has no place list and would not find the city
      either; `default_state` (item 30, decision 24) is the lever.
    - A CRF would add a Python training toolchain, a shipped model file and a
      retraining step for every fix, and could move other parses. The custom
      layer (decision 22) is the customization path.
    Consequences: items 39 to 42 and item 52 are dropped; `confidence` is
    calibrated on the rule paths only (decision 12); `model_version` and the
    empty `crf` cargo feature were removed (decision 35, item 53). **Reopen if** the hand-labeled Florida
    sample (item 36) or real use shows a persistent class of parses that
    rules and the place list cannot reach.

### Decisions recorded 2026-10-16 (documentation review)

Made by the author while reviewing the plan after Milestone 6.

33. **Publish names.** crates.io treats `us-address-parser` and
    `us_address_parser` as one name, and an MIT crate `us_address_parser`
    (v0.3.0, a different project) already holds it, so the Rust crates are
    published as `us-addr-core` and `us-addr-cli` (both free when checked).
    The Python package stays `us-address-parser` on PyPI (free), import
    `us_address_parser`; the CLI binary stays `addr`. This replaces the Rust
    part of decision 2. Item 45 does the rename and checks again before
    publishing.
    **Replaced by decision 42 (2026-10-17):** the names are dropped to avoid
    any confusion with the `us_address_parser` crate; item 58 chooses one name.
34. **`k1` freezes at the first release, not before.** Until the first
    release a parser fix may change the key an input produces (`SR 51`
    becoming `STATE ROAD 51` in Florida is one); each change is logged, dated,
    in `docs/key-format.md`. From the first release, any change to what an
    input produces needs `k2` and a docs update, as `CLAUDE.md` says. This
    corrects the earlier wording that called `k1` frozen already.
35. **Remove the unused surface now.** `Diagnostics::model_version` (always
    `None`) and the empty `crf` cargo feature go before any release, since
    nothing is published and decision 32 dropped the CRF. Item 53.
36. **Trailing country is recognized and ignored.** `USA`, `US`, `UNITED
    STATES` after the address are dropped from the parse with a warning
    `country_ignored`, instead of landing in `unparsed`. Other countries stay
    out of scope. Item 54.
37. **Intersections get a warning only.** `MAIN ST & 1ST AVE` raises
    `intersection_detected`; the second street stays in `unparsed` and the key
    stays `None`. Cross-street fields and a key form are revisited only if real
    data shows the volume. Item 55.
38. **Order, and the other two answers.** Milestone 8 runs before Milestone 7
    because the author is labeling the Florida sample (item 36); this order
    was replaced by decision 41 once items 53 to 56 and 43 were done. A city that is
    no known place keeps `place = None` even when the ZIP is known (the
    national list and the pack scripts now cover most communities, and a
    filled name could mislead).

### Decisions recorded 2026-10-17 (outside sources)

39. **Outside-source testing waits for v2 (reversed for Netsyms by decision 40).** NAD, OpenAddresses and the
    Netsyms Address Database were evaluated (`docs/future-sources.md`); none
    was downloaded or parsed. The first release ships on the accuracy already
    measured (96.1% on the blind benchmark, 99.2% on usaddress's files, the
    381-case corpus, about 900k fuzz runs), which covers street addresses
    mostly in Florida and one-line uppercase input, not units or mixed case
    elsewhere. No further source-driven parser rule work is scheduled before
    release: such changes would alter keys after `k1` is public, so they wait
    for `k2` (decision 34). A real bug found before release may still be
    fixed under the `CLAUDE.md` rule (log the key change). Disk (about 47 GB
    for Netsyms) is also not available. Plan: `docs/future-sources.md`.

40. **Reversal of decision 39 for the Netsyms data (2026-10-17).** The author
    freed the disk and chose to run the Netsyms full file (2026-05, 176M rows,
    956 US sources) before the first release, accepting parser changes now:
    until release a fix may change a key (`CLAUDE.md`), logged in
    `docs/key-format.md`. Done: `tools/netsyms/` (`sample.py`, `profile.py`,
    `compare.py`), eight parser fixes with corpus `ns-001` to `ns-023`
    (invented addresses; the data stays in `local-data/`), and the results in
    `docs/future-sources.md`. NAD direct and OpenAddresses stay deferred to v2
    (`k2`). Decision 39's other points stand: private data is never committed,
    and only numbers and our own fixes ship.

### Decisions recorded 2026-10-17 (release order)

41. **Milestone 7 before the rest of Milestone 8.** Decision 38 ran
    Milestone 8 first while the Florida sample was being labeled. Its parser
    and hardening items (53 to 56, 43) are done; what is left (44 to 46) is
    the release itself, and should follow Milestone 7:
    - The Florida sample (item 36) is a blind test on the data this library
      targets. A parser bug it finds changes keys; before the first release
      that is a logged fix, after it a `k2` (decision 34).
    - Decision 27 already put calibration before the first release; a first
      release with `confidence` always `None` would ship an empty feature.
      *Reversed by decision 43: a coarse score would be worse than none.*
    - The Pub 28 recheck (item 44) is meant to cover the final tables, and
      fixes from the sample may add spellings.
    - Item 38 was to add a fitted table to `data/`, which item 45 copies
      into the crate. *Dropped by decision 43.*
    - Decision 32 (no CRF) is to be reopened if the sample shows a class of
      parses rules cannot reach; that answer is wanted before publishing.
    Order: 37, 36, 38, 44, 45, 46. Milestone 9 may run beside them, and item
    45's rename and packaging checks may be tried early and redone after
    item 38.

### Decisions recorded 2026-10-17 (before release)

42. **Release preparation and a fresh start.** Made by the author:
    - **Benchmarks before release** (item 57): accuracy, speed and size
      measured and documented, with usaddress as one reference point. The
      docs keep the benchmarks and feature parity but stop framing the
      library as a competitor to usaddress (`docs/repository_cleaning.md`
      section 4).
    - **A new name** (item 58): decision 33's names are dropped because
      `us-address-parser` is too close to the existing `us_address_parser`
      crate; one name is researched and chosen for all packages, then
      applied by item 45.
    - **Examples and a customization guide** (items 59 and 60), and a custom
      layer as broad as data can reach (item 47).
    - **Text cleaning before release** (item 61): build-process language out
      of the docs (a historical outline stays in this file), the private
      Florida data sources not named (they are public record, but the
      released text describes them generically), required attributions kept,
      a user-focused README with diagrams. All deferred text work lives in
      `docs/repository_cleaning.md`.
    - **A fresh repository** (item 62): the cleaned, renamed tree goes into a
      new repository with no history, and the first release is published
      from there.

### Decisions recorded 2026-10-17 (confidence deferred)

43. **The confidence score is deferred past the first release.** Made by the
    author after item 38's first fit (`tools/calibrate/fit.py`):
    - **Why.** The score claims "the whole parse is right", which needs
      labeled answers. The only labeled data is the golden corpus, usaddress's
      test files (synthetic, 99.6% right) and the 190-row Florida sample, in
      all 52 wrong parses. Fitted on that, the score has six features and
      separates clean parses (about 0.98) from a few suspect kinds
      (0.58 to 0.87) and nothing finer; a score that looks precise and is not
      would be worse than none. The large private files (DBPR, auction,
      Netsyms) have no answers to score against.
    - **First release.** `Diagnostics::confidence` (Rust) and `confidence`
      (Python, stub) stay in the API and are always `None`; the docs say so.
      The field is kept so that adding the score later changes no signature.
      Decision 41's argument against shipping an always-`None` field is
      reversed: an honest `None` beats a coarse number.
    - **Kept** (small, no output change, needed by any later fit): the
      tagger's decisions (item 37) and their export (`Diagnostics` internal
      field, `doorplate corpus features`, `doorplate parse --features`,
      `Corpus::features`), `tools/calibrate/` (labeling tools and `fit.py`),
      and the Florida sample in `local-data/`.
    - **Dropped from the release path:** the fitted table in `data/`, its
      `build.rs` step, `docs/confidence.md`, the confidence line of
      item 57's benchmarks, and item 38's place in the order of decisions
      41 and 42.
    - **Repository changes this implies** are done or listed in
      `docs/repository_cleaning.md` section 8.
    - **Reopen** with the backlog entry "Calibrated confidence".

### Decisions recorded 2026-10-17 (release plan)

44. **The release plan and its order.** Made by the author, after the
    confidence score was deferred (decision 43):
    - **Sample bugs first (done).** The last two fixable errors from the
      Florida sample (two house numbers; box words before a numbered street,
      with the spaced `N W` directionals they exposed) land before any
      packaging, because they change keys and `k1` freezes at the release
      (`docs/key-format.md`, corpus `rg-137` to `rg-144`).
    - **Milestone 9 in the first release:** items 47 to 49 (the custom layer,
      its provenance in output, CLI and Python access), 50 (dataframe
      helpers) and, describing them, 59 (examples) and 60 (customization
      guide). Item 51 (display and long-form emitters) is not in the first
      release. *Narrowed by decision 45: items 47 to 49 and 60 are deferred.*
    - **Order:** 47, 48, 49, 50, 59, 60 (now 50, 59 only, decision 45); then 44 (Pub 28 recheck over the
      final tables), 57 (benchmarks), 58 (the name; the author's research
      may start any time), 45 (rename and packaging), 61 (text cleaning),
      62 (fresh repository), 46 (publishing). Examples and the guide come
      before the rename because item 45's rename map covers them.
    - Not in the first release: the confidence score (decision 43), item 51,
      NAD and OpenAddresses (v2, decision 39).

### Decisions recorded 2026-10-17 (custom layer deferred)

45. **The custom layer is deferred past the first release.** Made by the
    author, who has no layer to test:
    - **Why.** A layer built without a real user's data would be designed by
      guessing, and nothing in the first release depends on it. The core is
      already ready for it: `Lexicons` is the one interface, the tagger has no
      layer-specific logic, and `k1` does not change with a layer.
    - **Out of the first release:** items 47 (loader), 48 (provenance), 49
      (CLI and Python access) and 60 (customization guide). Item 59
      (examples) stays, with no layer example. Decision 44's order becomes 50,
      59, 44, 57, 58, 45, 61, 62, 46.
    - **Kept:** section 11 "Custom layer" and decision 22 as the design to
      build from; the README and comparison say what users do instead (report
      the input so the shipped tables are fixed; the golden corpus takes the
      case).
    - **No API break later.** `Parser::with_custom_layer` and
      `Parser(custom_layer=...)` are new optional entry points.
      `Diagnostics::custom_layer` is a new field on a struct that already has
      a private field, so no code outside the crate can construct it or match
      it without `..`; adding it is not a semver break.
    - **Reopen** with the backlog entry "Custom layer".

### Decisions recorded 2026-10-17 (the name)

46. **The name is `doorplate`.** Chosen by the author from the options in
    `docs/naming.md` (item 58), which also has the checks and the options
    ruled out:
    - **Names:** `doorplate` (Rust core crate, Python package and import,
      repository), `doorplate-cli` (CLI crate) with the binary `doorplate`,
      and `addr` kept as an alias binary built from the same code
      (`crates/doorplate-cli/src/bin/addr.rs`). Directories are
      `crates/doorplate`, `crates/doorplate-cli`, `crates/doorplate-py` and
      `fuzz/` (`doorplate-fuzz`). The key prefix stays `k1:`; the name is not
      part of the key format.
    - **Order change:** the rename (item 45, first half) runs before items 44
      and 57, so the recheck and the benchmarks use the final names. The
      packaging half of item 45 (data inside the crate) stays where it was.
    - **Still the author's:** a USPTO clearance search and a placeholder
      reservation on crates.io and PyPI before publishing. The web search in
      `docs/naming.md` is not a trademark search.
    - **History:** older decisions and `docs/naming.md` keep the old names
      (`addr-core`, `us-address-parser`) where they describe what was
      decided then; item 61 removes old names from the text.

### Open decisions (for the author)

Intersections, the trailing country and the city that is no known place were
decided (decisions 36 to 38). These remain:

- **The name** (item 58): from the short list in `docs/naming.md`.
- *Decided (decision 44):* the Milestone 9 items in the first release.

- **Recipient and name lines, landmarks, `CornerOf`, `NotAddress`:** stay
  unmodeled. Recommend documenting them (decision 25, done in the README) and
  letting the existing `unparsed_remainder` warning carry the signal.
  `attention` keeps covering ATTN and C/O (decision 7 of 2026-10-15). Say if
  you want any of them modeled.
- **Mixed-case display form** (`123 N Main St`) and a **long form**
  (`123 North Main Street`): `to_usps()` is uppercase by Pub 28 and the
  cleaner uppercases input, so there is no display casing; the tables have
  `Directional::name()` and `StreetSuffix::name()`, so both emitters are
  small. Neither is a Pub 28 form (label them display only). Recommend
  deferring until asked for; Milestone 9, item 51.

### Backlog (not scheduled)

Known work with no place in section 10 yet. Move an item into section 10
when it is scheduled.

- **Custom layer** (items 47 to 49 and 60, deferred by decision 45). Build it
  when someone has a real layer to test: a local community the place list
  lacks, a street spelling, a state-scoped suffix reading. Design: section 11,
  "Custom layer", and decision 22; the Milestone 9 entries keep the item text.
  Check the list of tables read through `Lexicons` (item 47) at that time,
  because they will have changed.

- **Florida code** from the author **(author)**, to seed more corpus cases.
- **The hand-labeled Florida sample** (item 36, **author**) is scheduled; it
  is listed here too because it gated Milestone 7 and so the first release
  (decision 41); it is done, and is the base for the deferred confidence
  score (below).
- **Calibrated confidence** (item 38, deferred by decision 43). Needs
  labeled answers at a scale the project does not have. In order of value:
  1. *Stratified hand labeling of the rare paths* (`bare_unit`,
     `city_by_suffix`, `number_letter`, `unparsed_remainder`,
     `missing_street_number`, the other warnings), drawn by path so that
     failures are not rare in the sample and weighted back when fitting;
     about 200 labeled rows would add more than 2,000 random ones. The
     collaborative mode in `tools/calibrate/README.md` fits this work.
  2. *Weak labels from structured sources*: DBPR keeps line 1, line 2, city,
     state and ZIP in separate columns, and Netsyms also the street number
     and name. Join a record into one string, parse it and compare with the
     stored fields: tens of thousands of automatic labels for those
     components, not for units or attention lines. The score would then claim
     "city, state, ZIP and street number are right" until a whole-parse
     label exists. Decision 12 says to fit on licensed data only; using DBPR
     (approved as a corpus source) or Netsyms (no formal license) for fitted
     numbers needs the author's decision first.
  3. Keep the geocode benchmark as the check, never the fit.
  What the first attempt showed: constrain every coefficient to at most
  zero, or the fit rewards features of the synthetic files; weight the
  Florida sample (half the weight worked); an exact-path table did no better
  than the logistic. Ship only if the held-out calibration is within a few
  points per band; otherwise leave `None`. Whatever ships adds the fitted
  table to `data/` (compiled by `build.rs`), fills `Diagnostics::confidence`
  and adds `docs/confidence.md`; filling a field that was always `None` is
  not a key-format change.
- **Accuracy, not tied to packs** (`docs/usaddress-comparison.md`): named
  buildings without a building word (`THE GARRETT`), highway modifiers such as
  BUSINESS before a city on one line, a comma after a directional
  (`3324 N, ANDREWS AVE`, 17 DBPR rows), a PO Box on line 2 beside a street
  with no number, a unit followed by business words (`STE 903 BLU SPA`, left
  unparsed), and `possibly_truncated` firing on complete units.
- **Outside-source benchmark and place-list gaps** (v2, decision 39;
  `docs/future-sources.md`): NAD or Netsyms rows rendered in several styles,
  compared per state and per source, feeding corpus cases, place-list gaps and
  pack priorities.
- **A batch call for pre-split fields** (`parse_fields_many`) and a columnar
  or Arrow result for dataframes: reading attributes from Python objects is
  now most of the time in a Polars or pandas pipeline (`crates/doorplate-py/README.md`).
- **Other state-pack rules** (section 11): `RR` as Ranch Road in Texas,
  `PR 462` (Texas private road or Puerto Rico), `LEE ROAD 2004`-style
  county road names, Queens-style hyphenated house numbers, Puerto Rico
  urbanizations. Each waits for real data from that state.
- **Pack infrastructure** (section 11): the pack version in output, a
  required corpus per pack enforced in CI (`doorplate pack test`).
- **Outside this repository:** drop `BELLVIEW` from BELLEVIEW's variants in
  auction-tracker's `cities_fl.json` (Bellview is a Census place in
  Escambia County). The pack's export already drops it.

---

## Sources

- us-addrs: https://github.com/raphaellaude/us-addrs
- us_address_parser: https://lib.rs/crates/us_address_parser
- rust-postal (`postal`): https://lib.rs/crates/postal
- rustpostal: https://docs.rs/crate/rustpostal/latest/source/README.md
- libpostal-rs: https://docs.rs/crate/libpostal-rs/latest/source/src/ffi.rs
- libpostal-sys: https://docs.rs/crate/libpostal-sys/latest
- crfs: https://lib.rs/crates/crfs
- usaddress (datamade): https://github.com/datamade/usaddress
- usaddress-scourgify: https://github.com/GreenBuildingRegistry/usaddress-scourgify
- libpypostal: https://pypi.org/project/libpypostal
- PyO3 and projects using it: https://www.github.com/PyO3/pyo3
