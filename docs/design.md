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
cases, all passing), the `addr` CLI and benchmarks. From Milestone 8, the
usaddress comparison is done. Milestone 5 is in progress: the Florida pack
is built and used (271 corpus cases, all passing). **Next: the national
place list, the benchmark rerun and the CRF call, calibrated confidence,
then Milestone 6, the Python wrapper.**
Section 10 has the plan and section 13 the open items.

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
| 1 | Parsing approach | Rules first; CRF fallback behind a cargo feature (`crf`, declared but empty). Whether to build it is decided after the place list (Milestone 5; section 13, decision 10). |
| 2 | Model training | Train in Python (parserator / python-crfsuite) on usaddress's MIT-licensed training data plus own labels; Rust does inference only via `crfs`. |
| 3 | Input scope | Both single-line strings and multi-field input (line1, line2, city, state, zip). |
| 4 | Geographic scope | US-wide grammar. Place data in layers (section 11): a national place list generated from public data, plus curated state packs, Florida first (Milestone 5). Other pack contents (road patterns) are deferred. |
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
  addr-py/                  # PyO3 bindings (planned, Milestone 6; README only)
data/                       # suffixes, directionals, units, states, special (.toml)
benches/parse.rs            # criterion benches
tests/
  corpus/                   # golden corpus (JSONL) and passing.txt baseline
  external/usaddress/       # usaddress's labeled test files (MIT, vendored)
tools/
  corpus/emit_expect.py     # writes emitter expectations into corpus cases from the specs
  compare/                  # usaddress comparison and training-overlap scripts
  train/                    # (planned, Milestone 7) CRF training scripts
models/                     # (planned, Milestone 7) CRF model; README only
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
- **Never panic on input.** Covered by proptest properties (`tests/no_panic.rs`); fuzz targets are Milestone 8.
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
version, `confidence` (`None` until calibrated; meaning in section 13,
decision 12) and `model_version` (`None` until a CRF model exists).

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

## 7. Python wrapper (`addr-py`, Milestone 6, not started)

- PyO3 with maturin; **abi3 wheels**, minimum Python **3.10**.
- Python import name `us_address_parser` (section 13, decision 2).
- Typed Rust-backed objects exposing the same fields, plus `to_dict()`. Ship type stubs (`.pyi`) and `py.typed`.
- `parse(str)`, `parse_fields(...)`, and **`parse_many(list[str])`**, which releases the GIL during Rust work.
- Emitters: `to_usps()`, `to_reso()`, `key()`, `slug()`, `to_usaddress_tags()`.
- `place` (name, state, county, kind, source) or `None`, as the CLI prints it.
- `confidence` is exposed as `float | None` (calibrated in Milestone 5; `None` only where no score applies, such as a placeholder).
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
| **Fuzzing** (cargo-fuzz) for the tokenizer and assembler. | Milestone 8. |
| **Python tests** (pytest): the wrapper, type stubs, `parse_many`; wheel builds on Linux, macOS and Windows. | Milestone 6. |
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

### Milestone 5: place list and calibrated confidence (next)

The first state-pack work (section 11), scoped to what the parser needs to
find the city. It comes before the Python wrapper, so Python ships with the
calibrated confidence, and it decides Milestone 7.

23. Place list data in two layers (section 11; section 13, decisions 13
    to 19):
    - **Florida pack** *(done)* (`data/packs/fl/`, feature `state-fl`), built by
      `tools/packs/build_fl.py` from auction-tracker's `cities_fl.json`
      (places, county, kind, main and shared ZIPs, variant spellings
      tagged as abbreviation or typo) and checked against its Census and
      GeoNames sources. Provenance and licenses in `docs/packs/fl.md`.
    - **National base** *(next)* (`data/places/`, feature `places-us`), built by
      `tools/places/build_us.py` from the Census 2024 Gazetteer (places
      and county subdivisions, public domain) and the GeoNames US postal
      file (ZIP to city, CC BY 4.0). Abbreviated spellings (`ST`/`SAINT`,
      directionals, Pub 28 suffix words inside names, `TWP`) come from a
      comparison rule, not stored variants. Built after the Florida pack,
      in the same format.
24. Use it behind the `Lexicons` interface (lookups take the state found
    from a state token or the ZIP's pack; no state logic in the tagger).
    *Done for Florida* (`docs/packs/fl.md`; corpus `fl-001` to `fl-013`):
    - find where the city starts on one line with no commas, and separate
      cities run into the street (`13551 WILSON STREETDADE CITY`); on a
      whole line only names and abbreviation variants count, typo
      variants only in a city field or when the ZIP agrees;
    - tell a street from a city before a ZIP (`WINGATE ST 32507`);
    - complete a cut-off city when exactly one place for the ZIP starts
      that way (`WESLEY CHA`);
    - fill a missing city from the ZIP's main place, and warn when city
      and ZIP disagree.
    Golden-corpus cases for each shape.
25. Calibrated confidence (section 13, decision 12): record the uncertain
    decisions the tagger makes, fit the score on licensed labeled data, and
    report calibration in `docs/`.
26. Rerun the usaddress comparison and the blind geocode benchmark; record the
    results in `docs/usaddress-comparison.md`, then make the Milestone 7
    go/no-go call (section 13, decision 10).

### Milestone 6: Python wrapper

27. `crates/addr-py/Cargo.toml`, `pyproject.toml` (maturin), `src/lib.rs`; add to the workspace
28. Python package `us_address_parser`: `__init__.py`, `__init__.pyi`, `py.typed`
29. `crates/addr-py/tests/test_basic.py`, `test_batch.py`
30. `.github/workflows/wheels.yml`

### Milestone 7: CRF fallback (only if Milestone 5 leaves a gap worth it)

31. First, verify that a CRFsuite-format model trained from usaddress data loads in `crfs`; if not, pick a compatible export path before investing further.
32. `tools/train/`: export labeled data, train with parserator/python-crfsuite, write the model file
33. `models/`: model file plus `README.md` with provenance, version, training date (usaddress MIT attribution)
34. `src/tagger/crf.rs` using `crfs`, behind feature `crf`; embed the model with `include_bytes!`. The CRF's sequence probability feeds `confidence`, calibrated the same way.

### Milestone 8: hardening and release

35. usaddress comparison script and results. *Done: `tools/compare/compare_usaddress.py`, `docs/usaddress-comparison.md`.*
36. Re-check every lexicon table and the USPS emitter rules against the live Pub 28 pages (`docs/usps-pub28-notes.md`, "Verification status").
37. Fuzz targets (cargo-fuzz) for the tokenizer and assembler.
38. Rename for publishing: crates and Python package to `us-address-parser` / `us_address_parser` (section 13, decision 2); check name availability first.
39. Copy `data/` into the `addr-core` crate for packaging (`build.rs` already looks for a crate-local `data/` first).
40. crates.io and PyPI publishing workflows; turn the CHANGELOG's "Unreleased" into the first release.

---

## 11. Deferred feature: state tuning packs

Mostly deferred. The place list (Milestone 5) is the first piece to be built;
the rest waits. Recorded so the core stays ready for it.

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
  overrides.toml   # (later) SR/CR and other road patterns
```

Layering is add-only: a later layer adds names, variants and ZIPs and
never replaces or removes an earlier entry; a conflict fails the build.
Packs are fixed at compile time (section 3, decision 7); there are no user
packs loaded from a directory. State selection is two-pass: find the
state (state token or ZIP prefix), then look up places in that state.

**Work already waiting on packs** (from the corpus, `docs/prior-art.md` and
`docs/usaddress-comparison.md`):

- A place list to find the city on one line with no commas (`ST LOUIS`,
  `MOUNTAIN VIEW`, `W. PALM BEACH`). *Done for Florida*, along with a city
  run into the street (`13551 WILSON STREETDADE CITY`), cut-off cities
  completed from the ZIP (`WESLEY CHA`), the place from the ZIP, and
  `WINGATE ST 32507` (street plus ZIP, not city plus ZIP). Nationally this
  is the largest remaining gap on the blind benchmark; the Census 2024
  Gazetteer (public domain) covers 73.5% of its cities.
- `SR` (State Road in Florida, State Route elsewhere), `RR` as Ranch Road in
  Texas, `PR 462` (Texas private road vs Puerto Rico), `CTY ROAD`,
  `LEE ROAD 2004`-style county road names.
- Queens-style hyphenated house numbers, Puerto Rico urbanizations.
- `--default-state` for `addr parse-file` (most of the 1,700 auction rows
  with no state).
- `ST`/`FT`/`MT` vs `SAINT`/`FORT`/`MOUNT` in place names, and suffix
  abbreviations inside place names (`ALTAMONTE SPGS`). *Done*: the place
  comparison rule (`lexicon/place_key.rs`).

Settled 2026-10-16 (section 13, decisions 13 to 19): add-only layering,
compile-time packs, the pack version recorded in `pack.toml` but not yet in
output, and GeoNames (CC BY 4.0) as the ZIP source. Still open:

- Whether packs may ship their own model weights (suggest no initially; reserve a manifest field).
- A required golden-test corpus per pack, enforced in CI (`addr pack test`).
- Reporting the pack version in output.

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
  must be verified before any Milestone 7 work.
- **Key stability.** `k1` is signed off and stored keys depend on it. Parser
  fixes can still change the key an input produces; each such change is
  recorded in `docs/key-format.md`.
- **Licensing.** Keep MIT attribution for vendored usaddress test data and
  any usaddress-derived training data. Unlicensed test data (the geocode
  benchmark) stays in `local-data/` and is never committed.
- **Fuzzing** does not exist yet (Milestone 8); proptest covers the
  never-panic rule meanwhile.
- **Python boundary overhead** for single-address calls; recommend `parse_many` in the docs.

---

## 13. Decisions and open items

### Decisions recorded 2026-10-15

1. **Florida parsing code: pending.** The author will provide it later, as a reference (not ported). It seeds the corpus when it arrives.
2. **Names: `us-address-parser`.** Publish names (crates.io and PyPI; Python import `us_address_parser`) will be `us-address-parser`; the CLI binary stays `addr`. The rename applies at Milestone 8; until then the crates keep the `addr-core`, `addr-cli` and `addr-py` names.
3. **Licensing: confirmed.** MIT OR Apache-2.0.
4. **Key definition: signed off.** `k1` (section 6) is the public contract; any change needs a new version prefix. **Minimum Python version: 3.10** (abi3 wheels).
5. **RESO mapping: reviewed.** All choices accepted as implemented; see "Decisions" in `docs/reso-mapping.md`.
6. **Dual addresses (Pub 28 section 215): hold both.** A record with a street address and a PO Box keeps both (street fields and `po_box`, warning `dual_address`). The key and the RESO output use the street; the USPS output writes the PO Box alone, as section 215 prefers. Rejected: keeping one and leaving the other unparsed, and returning two addresses.
7. **Attention lines: a field.** `ParsedAddress::attention` holds an ATTN or C/O line (section 214). No emitter writes it.
8. **Rule changes from the usaddress comparison and geocode benchmark** (`docs/usps-pub28-notes.md`, rows added 2026-10-15): a no-ZIP state at the end of one line (never NE or FL; suffix-like codes only after an abbreviated suffix); hyphenated routes split (`GA-25`), SR left to state packs; `BOX <number>` alone is a PO Box; a letter before a directional joins the house number; letter-and-digits house numbers (`T703`); two-part grid numbers kept as written; letter routes after a county prefix; `COUNTRY ROAD` before a number is an observed typo; BUSINESS after a route stays in the name; `missing_street_name` for a number and unit with no street; ordinal floors normalized to numbers (`FL 5`); `STE 4 551` stays unparsed; a `building` field.
9. **Deferred to state packs:** the items listed in section 11.

### Decisions recorded 2026-10-16

10. **CRF fallback: decided after the place list.** Milestone 5 builds the
    place list and reruns the blind benchmark; Milestone 7 goes ahead only if
    a gap remains that rules plus the place list are unlikely to close.
11. **Slug for dual addresses: follow the street.** The key and the RESO
    output already use the street; only the USPS output writes the PO Box,
    because Pub 28 section 215 says so for the mailpiece. The slug will be
    built from the street line, as the RESO emitter's `UnparsedAddress` is
    (the shared `street_line()` helper in `emit/usps.rs`), so every
    identifier names the same place. The slug is not a contract, so this
    needs no version change. Planned before the Python wrapper (Milestone 6),
    since Python exposes the slug.
12. **`Diagnostics::confidence`: a calibrated probability.** A number from
    0.0 to 1.0: the chance that the whole parse (every component) is right,
    calibrated so that of the addresses scored 0.9, about 90% are right on a
    labeled set the rules were not tuned on. It stays `None` until it is
    calibrated (Milestone 5, item 25). Plan:
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
    - If a CRF is built (Milestone 7), its sequence probability gives the
      score for CRF parses, checked against the same calibration report.
    - Calibration waits for the place list, because the place list changes
      which paths are uncertain.

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

### Open decisions (for the author)

- **A city that is no known place, with a ZIP the pack knows** (`SINGER
  ISLAND`, `SAINT LUCIE COUNTY`): `place` is `None` today. auction-tracker
  uses the ZIP's place there. Filling it (source `zip`) would name the
  place mail goes to, but would put a different name beside a real
  community's.

### Open tasks

- **Slug follows the street for dual addresses** (decision 11): change
  `emit/slug.rs`, the slug section of `docs/key-format.md`, the corpus slug
  expectations for `rg-040` to `rg-045` and `tools/corpus/emit_expect.py`.
- **Milestone 5**, in order: the national base list (next), the benchmark
  rerun and the CRF call, then calibrated confidence. The Florida pack is
  done.
- **auction-tracker's `cities_fl.json`**: drop `BELLVIEW` from BELLEVIEW's
  variants (Bellview is a Census place in Escambia County). The pack's
  export already drops it (`docs/packs/fl.md`).
- **Milestone 6**, the Python wrapper.
- **Florida code** from the author, to seed more corpus cases.
- **Blind Florida sample:** hand-label 100 to 200 rows of the private Florida
  files. It is a blind test on the data this library targets and calibration
  data for confidence.
- **Accuracy backlog** not tied to packs (`docs/usaddress-comparison.md`):
  named buildings without a building word (`THE GARRETT`), and highway
  modifiers such as BUSINESS before a city on one line.
- **Milestone 8** items 36 to 40 (Pub 28 re-check, fuzzing, rename and name
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
