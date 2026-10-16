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
| `docs/prior-art.md` | Lessons from the author's Florida address code |
| `tests/corpus/README.md` | Golden-corpus record format and labeling conventions |

**Status (2026-10-16):** Milestones 0 to 4 are done: data and types, the
rule parser, the key, USPS, RESO and slug emitters, the golden corpus (258
cases, all passing), the `addr` CLI and benchmarks. From Milestone 7, the
usaddress comparison is done. **Next: Milestone 5, the Python wrapper.**
Section 10 has the plan and section 13 the open items.

---

## 1. Goal and scope

Build a small, dependable library that:

1. Parses US address strings (and pre-split address fields) into components.
2. Emits normalized output in **USPS Publication 28** style and **RESO Data Dictionary** field names.
3. Produces **database-ready identifiers**: a stable canonical match key and a URL-safe slug.
4. Reports **diagnostics** (warnings, unparsed remainder) so bulk users can route bad rows to review.
5. Is usable from Rust directly and from Python via a PyO3 wrapper, with no C library or data download required at runtime.

The author's Florida address code (auction-tracker, dbpr_fl) is the main
source of domain knowledge; what carried over is in `docs/prior-art.md`.

### Explicit non-goals (v1)

- **Address validation or deliverability.** Pub 28 formatting is not the same as a deliverable address; real validation needs CASS-certified data. The README states this plainly.
- Geocoding, fuzzy matching, address correction, international addresses.
- ZIP-to-city or county lookup tables (possible later via state packs, see section 11).
- Runtime-configurable lexicons (fixed at compile time in v1).

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
| 1 | Parsing approach | Rules first; CRF fallback behind a cargo feature (`crf`, declared but empty). Whether to build it is open (section 13). |
| 2 | Model training | Train in Python (parserator / python-crfsuite) on usaddress's MIT-licensed training data plus own labels; Rust does inference only via `crfs`. |
| 3 | Input scope | Both single-line strings and multi-field input (line1, line2, city, state, zip). |
| 4 | Geographic scope | US-wide grammar. **State-specific tuning packs are deferred** (section 11); the core is designed so packs can be added without restructuring. |
| 5 | Canonical key | Unit inclusion is an explicit option (default off, property level); the key carries a version prefix. City and ZIP+4 excluded. Spec in `docs/key-format.md`. |
| 6 | Error model | Always return a `ParsedAddress` with diagnostics; `Result` only for unusable input (empty). |
| 7 | Lexicon extensibility | Fixed at compile time in v1. |
| 8 | Data sources | USPS Pub 28 and the RESO Data Dictionary; provenance of each table documented. |
| 9 | Python surface *(default)* | Typed objects plus `to_dict()`; no pandas/Polars dependency. |
| 10 | Distribution *(default)* | abi3 wheels; library, model, and key-format versions tracked independently. |
| 11 | Compatibility | Own schema; optional usaddress-compatible tag output (`to_usaddress_tags()`, done). |
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
Cargo.toml                  # workspace: addr-core, addr-cli
rust-toolchain.toml, .cargo/config.toml   # stable toolchain; MSRV-aware resolver (MSRV 1.80)
README.md, CLAUDE.md, CHANGELOG.md, LICENSE-MIT, LICENSE-APACHE
docs/                       # this document and the specs listed at the top
crates/
  addr-core/                # pure Rust, no Python dependency
    build.rs                # compiles data/*.toml into static tables
    src/                    # clean, tokenize, classify, tagger/, assemble, parser,
                            # input, lexicon, types, emit/{key,usps,reso,slug,usaddress}, corpus
    tests/                  # corpus.rs, no_panic.rs (proptest), usaddress_gold.rs
  addr-cli/                 # `addr` binary: parse, parse-file, corpus test
  addr-py/                  # PyO3 bindings (planned, Milestone 5; README only)
data/                       # suffixes, directionals, units, states, special (.toml)
benches/parse.rs            # criterion benches
tests/
  corpus/                   # golden corpus (JSONL) and passing.txt baseline
  external/usaddress/       # usaddress's labeled test files (MIT, vendored)
tools/
  corpus/emit_expect.py     # writes emitter expectations into corpus cases from the specs
  compare/                  # usaddress comparison and training-overlap scripts
  train/                    # (planned, Milestone 6) CRF training scripts
models/                     # (planned, Milestone 6) CRF model; README only
.github/workflows/ci.yml    # fmt, clippy, test, corpus report
local-data/                 # gitignored: private and unlicensed test data, never committed
```

---

## 5. Core design (`addr-core`)

### Pipeline

```
raw input
  -> clean.rs     whitespace, accents, Unicode punctuation, uppercase; byte map to the original
  -> tokenize.rs  tokens with original spans, lines and comma segments, glued-token splits
  -> classify.rs  token shapes and longest-match lexicon lookups
  -> tagger/      rules (CRF fallback, if built, would go here)
  -> assemble.rs  ParsedAddress, component spans, warnings
  -> emit/        read-only consumers: key, usps, reso, slug, usaddress tags
```

Emitters never mutate the parsed struct and never re-parse.

### Principles

- **Deterministic.** Same input, same output, across platforms.
- **Spans preserved.** Components keep their source byte ranges (`SourceInfo::spans`).
- **Never panic on input.** Covered by proptest properties (`tests/no_panic.rs`); fuzz targets are Milestone 7.
- **Lexicon access behind one interface** (`Lexicons`, built from `data/*.toml` at build time). This is the seam state packs will plug into.
- No global mutable state; `Parser` is cheap and `Send + Sync`.

### `ParsedAddress`

Defined in `crates/addr-core/src/types.rs`. Fields: street number,
pre/post directional, street name, suffix (`StreetSuffix`), `highway`
(state, prefix, route), first and second unit (type and number), `po_box`,
`rural_route` (RR/HC, route, box), `general_delivery`, `military` and
`military_post_office`, `attention`, `building`, city, state (`StateCode`),
ZIP5, ZIP+4, `diagnostics` and `source`.

`Diagnostics` holds the warnings (`Warning`, each with a stable `code()`
string; `Warning::ALL` lists them), the unparsed remainder, the library
version, and `confidence` / `model_version`, which stay `None` until a CRF
model exists.

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

Cargo features: `rayon` (parallel `parse_many`), `corpus` (the golden-corpus
runner, pulls in serde), `crf` (reserved). The default build has no runtime
dependencies.

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

## 7. Python wrapper (`addr-py`, Milestone 5, not started)

- PyO3 with maturin; **abi3 wheels**, minimum Python **3.10**.
- Python import name `us_address_parser` (section 13, decision 2).
- Typed Rust-backed objects exposing the same fields, plus `to_dict()`. Ship type stubs (`.pyi`) and `py.typed`.
- `parse(str)`, `parse_fields(...)`, and **`parse_many(list[str])`**, which releases the GIL during Rust work.
- Emitters: `to_usps()`, `to_reso()`, `key()`, `slug()`, `to_usaddress_tags()`.
- Errors map to Python exceptions only for unusable input.
- No required runtime dependencies. No pandas/Polars dependency; document a short recipe instead.

---

## 8. CLI (`addr-cli`, done)

- `addr parse "<address>"` prints one JSON record (components, warnings, key, slug, USPS and RESO output); `--pretty`, `--usps`, `--usaddress`, field options (`--line1` ... `--zip`), or one address per stdin line.
- `addr parse-file IN.csv --column NAME [--line2 ...] [-o OUT.csv]` appends `addr_`-prefixed columns.
- `addr corpus test [--dir DIR] [--verbose] [--update-baseline]` runs the golden corpus and prints the accuracy report.

---

## 9. Testing and quality

| Check | Status |
|---|---|
| **Golden corpus** (`tests/corpus/*.jsonl`): expected components and warnings; most cases also carry key, slug, USPS and RESO expectations written from the specs by `tools/corpus/emit_expect.py`. The runner fails on malformed records and on regressions against `passing.txt`. | Done: 258 cases, all passing. |
| **USPS round trip**: parsing the USPS output of a parsed address gives the same key. | Done (`tests/corpus.rs`). |
| **Property tests** (proptest): no panic, valid spans, every emitter, `parse_many` consistency. | Done (`tests/no_panic.rs`). |
| **usaddress gold score**: this parser on usaddress's labeled test files, with a floor. | Done (`tests/usaddress_gold.rs`). |
| **Benchmarks** (criterion): single shapes, the corpus through `parse` and `parse_many`, the emitters. | Done (`benches/parse.rs`). |
| **CI**: `cargo fmt --check`, `clippy -D warnings`, `cargo test --all-features`, corpus accuracy report. | Done (`ci.yml`). |
| **Fuzzing** (cargo-fuzz) for the tokenizer and assembler. | Milestone 7. |
| **Python tests** (pytest): the wrapper, type stubs, `parse_many`; wheel builds on Linux, macOS and Windows. | Milestone 5. |
| **Blind accuracy**: labels neither this parser nor usaddress was tuned on. | Geocode benchmark run locally (`docs/usaddress-comparison.md`); a hand-labeled Florida sample is still open. |

The corpus is hand-labeled and the rules were tuned against it, so 100% on
it says little about unseen data; the blind benchmark is the better measure.

---

## 10. Milestones

Each milestone ends with passing tests.

### Milestone 0: scaffolding (done)

1. Workspace `Cargo.toml`, `rust-toolchain.toml`, `.gitignore`, licenses
2. `README.md` (with the non-goals and the not-a-validator statement)
3. `CLAUDE.md`
4. `crates/addr-core` skeleton
5. `.github/workflows/ci.yml`

### Milestone 1: data and types (done)

6. `data/*.toml`, each with its source; verified against the live Pub 28 pages (`docs/usps-pub28-notes.md`)
7. `crates/addr-core/build.rs` (compile and validate the TOML into static tables)
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

19. Golden corpus seeded from auction-tracker's tests and golden CSVs, Pub 28's own examples and regression cases. The author's own Florida code is still pending (section 13).
20. Corpus runner: the `addr_core::corpus` module (feature `corpus`), used by `tests/corpus.rs` and `addr corpus test`. Exit codes: 0 ok, 2 malformed corpus, 3 regression or stale baseline.
21. `addr` CLI (section 8). The CLI, not the core, depends on `csv` and `serde_json`.
22. `benches/parse.rs`. First numbers (quick run, 2026-10-15): about 4 to 11 µs per address, about 110K addresses/s one by one, about 3x that through `parse_many` with `rayon`. Run `cargo bench -p addr-core --bench parse` (add `--features rayon`).

### Milestone 5: Python wrapper (next)

23. `crates/addr-py/Cargo.toml`, `pyproject.toml` (maturin), `src/lib.rs`; add to the workspace
24. Python package `us_address_parser`: `__init__.py`, `__init__.pyi`, `py.typed`
25. `crates/addr-py/tests/test_basic.py`, `test_batch.py`
26. `.github/workflows/wheels.yml`

### Milestone 6: CRF fallback (optional; go/no-go open, section 13)

27. `tools/train/`: export labeled data, train with parserator/python-crfsuite, write the model file
28. `models/`: model file plus `README.md` with provenance, version, training date (usaddress MIT attribution)
29. `src/tagger/crf.rs` using `crfs`, behind feature `crf`; embed the model with `include_bytes!`
30. First, verify that a CRFsuite-format model trained from usaddress data loads in `crfs`; if not, pick a compatible export path before investing further.

### Milestone 7: hardening and release

31. usaddress comparison script and results. *Done: `tools/compare/compare_usaddress.py`, `docs/usaddress-comparison.md`.*
32. Re-check every lexicon table and the USPS emitter rules against the live Pub 28 pages (`docs/usps-pub28-notes.md`, "Verification status").
33. Fuzz targets (cargo-fuzz) for the tokenizer and assembler.
34. Rename for publishing: crates and Python package to `us-address-parser` / `us_address_parser` (section 13, decision 2); check name availability first.
35. Copy `data/` into the `addr-core` crate for packaging (`build.rs` already looks for a crate-local `data/` first).
36. crates.io and PyPI publishing workflows; turn the CHANGELOG's "Unreleased" into the first release.

---

## 11. Deferred feature: state tuning packs

Not part of the initial build. Recorded so the core stays ready for it.

**Idea:** the base parser is US-wide and state-agnostic. Each state gets a data-only "pack" of tuning files, so accuracy can improve per state without touching core code. Florida would be first.

Sketch of a pack:

```
packs/fl/
  pack.toml        # state code, pack version, minimum core version, data provenance
  places.csv       # canonical place names, aliases, county, type
  zips.csv         # ZIP -> place(s), county
  overrides.toml   # suffix/abbreviation tweaks, road patterns
  tests/           # labeled addresses and golden outputs
```

Layering: base defaults, then state pack, then optional user pack from a directory; later layers win. Official packs could be embedded via cargo features (`state-fl`) and Python extras. State selection would be two-pass: find the state (state token or ZIP prefix), then tag with that state's pack; with no pack, fall back to base only.

**Work already waiting on packs** (from the corpus, `docs/prior-art.md` and
`docs/usaddress-comparison.md`):

- A place list to find the city on one line with no commas (`ST LOUIS`,
  `MOUNTAIN VIEW`, `W. PALM BEACH`), a city run into the street
  (`13551 WILSON STREETDADE CITY`), cut-off cities completed from the county
  (`WESLEY CHA`), and city from ZIP. This is the largest remaining gap on the
  blind benchmark. The Census 2024 Gazetteer (public domain) covers 73.5% of
  its cities; for Florida, auction-tracker's `cities_fl.json` and dbpr_fl's
  variant list.
- `WINGATE ST 32507` (street plus ZIP, or city plus ZIP): only a place list
  separates them.
- `SR` (State Road in Florida, State Route elsewhere), `RR` as Ranch Road in
  Texas, `PR 462` (Texas private road vs Puerto Rico), `CTY ROAD`,
  `LEE ROAD 2004`-style county road names.
- Queens-style hyphenated house numbers, Puerto Rico urbanizations.
- `--default-state` for `addr parse-file` (most of the 1,700 auction rows
  with no state).
- `ST`/`FT`/`MT` vs `SAINT`/`FORT`/`MOUNT` in place names, and suffix
  abbreviations inside place names (`ALTAMONTE SPGS`).

Open questions to settle when this is picked up:

- Overlay semantics (add only, or add/replace/remove).
- Pack versioning and core compatibility checks (include pack version in output).
- ZIP data source and licensing: Census ZCTAs are public but not identical to USPS ZIPs; confirm redistribution terms for any USPS-derived table.
- Whether packs may ship their own model weights (suggest no initially; reserve a manifest field).
- A required golden-test corpus per pack, enforced in CI (`addr pack test`).

**Design-now constraints:** keep all lexicon access behind the `Lexicons`
interface; keep the key independent of city (section 6); keep diagnostics
extensible; no state-specific rule in the tagger.

---

## 12. Risks

- **Accuracy parity with usaddress.** On the blind geocode benchmark (30,622
  one-line addresses) this parser gets 85.4% of whole addresses right
  (coarse) against usaddress's 88.2%; it does better where there is no city
  and worse where a suffix-less street runs into the city. On usaddress's own
  test files it gets 98.9% (those are usaddress's regression tests, so
  usaddress scores 100% by construction). Details:
  `docs/usaddress-comparison.md`.
- **Model format compatibility** between CRFsuite-trained models and `crfs`
  must be verified before any Milestone 6 work.
- **Key stability.** `k1` is signed off and stored keys depend on it. Parser
  fixes can still change the key an input produces; each such change is
  recorded in `docs/key-format.md`.
- **Licensing.** Keep MIT attribution for vendored usaddress test data and
  any usaddress-derived training data. Unlicensed test data (the geocode
  benchmark) stays in `local-data/` and is never committed.
- **Fuzzing** does not exist yet (Milestone 7); proptest covers the
  never-panic rule meanwhile.
- **Python boundary overhead** for single-address calls; recommend `parse_many` in the docs.

---

## 13. Decisions and open items

### Decisions recorded 2026-10-15

1. **Florida parsing code: pending.** The author will provide it later, as a reference (not ported). It seeds the corpus when it arrives.
2. **Names: `us-address-parser`.** Publish names (crates.io and PyPI; Python import `us_address_parser`) will be `us-address-parser`; the CLI binary stays `addr`. The rename applies at Milestone 7; until then the crates keep the `addr-core`, `addr-cli` and `addr-py` names.
3. **Licensing: confirmed.** MIT OR Apache-2.0.
4. **Key definition: signed off.** `k1` (section 6) is the public contract; any change needs a new version prefix. **Minimum Python version: 3.10** (abi3 wheels).
5. **RESO mapping: reviewed.** All choices accepted as implemented; see "Decisions" in `docs/reso-mapping.md`.
6. **Dual addresses (Pub 28 section 215): hold both.** A record with a street address and a PO Box keeps both (street fields and `po_box`, warning `dual_address`). The key and the RESO output use the street; the USPS output writes the PO Box alone, as section 215 prefers. Rejected: keeping one and leaving the other unparsed, and returning two addresses.
7. **Attention lines: a field.** `ParsedAddress::attention` holds an ATTN or C/O line (section 214). No emitter writes it.
8. **Rule changes from the usaddress comparison and geocode benchmark** (`docs/usps-pub28-notes.md`, rows added 2026-10-15): a no-ZIP state at the end of one line (never NE or FL; suffix-like codes only after an abbreviated suffix); hyphenated routes split (`GA-25`), SR left to state packs; `BOX <number>` alone is a PO Box; a letter before a directional joins the house number; letter-and-digits house numbers (`T703`); two-part grid numbers kept as written; letter routes after a county prefix; `COUNTRY ROAD` before a number is an observed typo; BUSINESS after a route stays in the name; `missing_street_name` for a number and unit with no street; ordinal floors normalized to numbers (`FL 5`); `STE 4 551` stays unparsed; a `building` field.
9. **Deferred to state packs:** the items listed in section 11.

### Open decisions (for the author)

- **CRF fallback (Milestone 6): go or no-go.** The remaining blind-benchmark
  gap is mostly city detection, which a place list addresses. Decide after
  the place list, or after a blind Florida sample, whether a CRF is still
  worth building.
- **Slug for dual addresses.** The slug follows the USPS output, so it names
  the PO Box while the key names the street. Left as specified; change it if
  the slug should follow the street.
- **`Diagnostics::confidence`.** Always `None` today. Define its meaning
  before the Python wrapper exposes it, or leave it out of the Python API.
- **State-pack questions** (section 11) when that work starts.

### Open tasks

- **Milestone 5**, the Python wrapper (next).
- **Florida code** from the author, to seed more corpus cases.
- **Blind Florida sample:** hand-label 100 to 200 rows of the private Florida
  files as a blind test on the data this library targets.
- **Accuracy backlog** not tied to packs (`docs/usaddress-comparison.md`):
  named buildings without a building word (`THE GARRETT`), and highway
  modifiers such as BUSINESS before a city on one line.
- **Milestone 7** items 32 to 36 (Pub 28 re-check, fuzzing, rename and name
  availability, packaging `data/`, publishing).

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
