# US Address Parser: Project Plan

A Rust-core US address parsing library with a Python wrapper. This document summarizes the planning conversation, records decisions, and lists the initial files to build. It is written to be dropped into a fresh repo and handed to Claude Code.

Status: Milestones 0 to 4 done (scaffolding, data and types, parser, emitters, corpus and CLI). Emitters: the key, USPS, RESO (Data Dictionary 2.0) and slug emitters. Progress is tracked in section 10. Planning date: 2026-10-14.

---

## 1. Goal and scope

Build a small, dependable library that:

1. Parses US address strings (and pre-split address fields) into components.
2. Emits normalized output in **USPS Publication 28** style and **RESO Data Dictionary** field names.
3. Produces **database-ready identifiers**: a stable canonical match key and a URL-safe slug.
4. Reports **diagnostics** (warnings, unparsed remainder, confidence) so bulk users can route bad rows to review.
5. Is usable from Rust directly and from Python via a PyO3 wrapper, with no C library or data download required at runtime.

The author already has a substantial body of Florida-specific address parsing code. That code and its test cases are the main source of domain knowledge and should seed the test corpus (see section 9).

### Explicit non-goals (v1)

- **Address validation or deliverability.** Pub 28 formatting is not the same as a deliverable address; real validation needs CASS-certified data. The README must state this plainly.
- Geocoding, fuzzy matching, address correction, international addresses.
- ZIP-to-city or county lookup tables (possible later via state packs, see section 11).
- Runtime-configurable lexicons (fixed at compile time in v1).

---

## 2. Research summary (as of October 2026)

Findings come from web research done during planning. Nothing below was verified by reading source code in depth; skim the small crates' tests before relying on them.

### Rust ecosystem

| Project | What it is | Assessment |
|---|---|---|
| `us-addrs` (raphaellaude) | Rust port of Python `usaddress`; claims ~5x faster | Closest prior art. Self-described as under active development, accuracy not yet matching usaddress, 6 stars, 35 commits. Not production-grade. |
| `us_address_parser` | Simple rule-based crate | Street number, direction, name, type, unit only. No city/state/ZIP. ~145 downloads/month. |
| `rustpostal`, `postal` (rust-postal), `libpostal-rs`, `libpostal-sys` | Bindings to libpostal (C) | Heavy: needs C library and large data files; libpostal is not thread-safe (single context per process); international and statistical, not tuned for USPS conventions. |
| `crfs` | Pure-Rust port of CRFsuite | **Inference only** (no training). Useful building block for running a usaddress-style CRF model in pure Rust. Check current release status when adding it. |

Conclusion: no mature pure-Rust US address parser with Python bindings exists. The gap is real.

### Python ecosystem (what we compete with)

- `usaddress` (datamade): the standard. CRF model via parserator and python-crfsuite. ~1,600 stars, ~174 open issues, MIT license.
- `usaddress-scourgify`: normalization to USPS Pub 28 and RESO guidelines; does no validation.
- libpostal Python wrappers (`pypostal`, forks, `libpypostal`): require the libpostal C library; the official one has seen little development for years.

### Rust core + Python wrapper pattern

Well proven: polars, pydantic-core, orjson, cryptography, tokenizers and others use PyO3, typically built and published with maturin. Address parsing fits well (CPU-bound, text-heavy, often run in bulk). Pitfalls: per-call boundary overhead can erase speed gains for single addresses, so expose batch APIs and release the GIL.

**Accuracy, not speed, is the bar to beat.** us-addrs shows how hard it is to match usaddress. Benchmark against usaddress on the Florida labeled corpus before claiming anything.

---

## 3. Architecture decisions

These were the open questions, with the resolved answers. Items marked *(proposed)* were my defaults that the author accepted by saying "defaults are fine"; revisit if they cause friction.

| # | Decision | Resolution |
|---|---|---|
| 1 | Parsing approach | Rules first, CRF fallback behind a cargo feature (`crf`). |
| 2 | Model training | Train in Python (parserator / python-crfsuite) on usaddress's MIT-licensed training data plus own Florida labels; Rust does inference only via `crfs`. |
| 3 | Input scope | Both single-line strings and multi-field input (line1, line2, city, state, zip). |
| 4 | Geographic scope | US-wide grammar. **State-specific tuning packs are deferred** until after the core parser is done (section 11). Design the core so packs can be added later without restructuring. |
| 5 | Canonical key | Unit inclusion is an explicit option (default off, property level); key carries a version prefix. City and ZIP+4 excluded. Confirmed by the author; spec in `docs/key-format.md`. |
| 16 | Key shape | Pipe-delimited uppercase ASCII with a one-letter form (`S`, `P`, `R`, `M`, `G`). `k1:` needs state and ZIP5; otherwise `k1n:`. Unit type is dropped at unit level so `Apt 4` equals `#4`. Placeholders and addresses with nothing to key on give `None`. |
| 6 | Error model | Always return a `ParsedAddress` with diagnostics; no `Result` for partial parses. `Result` only for truly unusable input (e.g. empty). |
| 7 | Lexicon extensibility | Fixed at compile time in v1. |
| 8 | Data sources | USPS Pub 28 and the RESO Data Dictionary; document provenance of each table. |
| 9 | Python surface | Typed objects plus `to_dict()`; no pandas/Polars dependency. |
| 10 | Distribution | abi3 wheels; library, model, and key-format versions tracked independently. |
| 11 | Compatibility | Own schema; optional usaddress-compatible tag output. |
| 12 | License | Dual MIT / Apache-2.0 (confirm if a paid tier is planned). |
| 13 | Component spans | Byte ranges into the **original** input, not the cleaned text. `clean.rs` keeps a map from cleaned to original offsets. For multi-field input a span also names the field it came from. |
| 14 | Multi-field input | `parse_fields` tags each field on its own, using the field as a strong signal (the city field is the city), instead of joining the fields into one line. Field quirks from real data are in `docs/prior-art.md`. A source that gives a street line plus a `CITY, ST ZIP` line passes them as `line1` and `line2`: when city, state and ZIP are all empty, the address lines are read like one line (settled in item 15). |
| 15 | Address shapes with no field yet | Add fields to `ParsedAddress` for highway routes (prefix kept apart from the route number), general delivery, military (PSC/CMR/UNIT box plus APO/FPO/DPO), and a second unit designator. The RESO emitter folds those it has no field for into `StreetName` or unit fields; document how in `docs/reso-mapping.md`. |

---

## 4. Repository layout

Cargo workspace with three crates plus data and tooling:

```
addr/
  Cargo.toml                  # workspace
  rust-toolchain.toml
  README.md
  CLAUDE.md                   # guidance for Claude Code (draft in appendix A)
  LICENSE-MIT
  LICENSE-APACHE
  CHANGELOG.md
  docs/
    design.md                 # this document, trimmed as decisions land
    usps-pub28-notes.md       # sourced tables and rules actually implemented
    reso-mapping.md           # field mapping, with DD version noted
    key-format.md             # key and slug spec, versioned
  crates/
    addr-core/                # pure Rust, no Python dependency
    addr-py/                  # PyO3 bindings, built with maturin
    addr-cli/                 # batch CSV and debugging tool
  data/
    suffixes.toml             # Pub 28 street suffixes and abbreviations
    directionals.toml
    units.toml                # secondary unit designators
    states.toml               # state names, USPS codes
    special.toml              # PO BOX, RR, HC, highway patterns, etc.
  models/
    README.md                 # model provenance, training date, version
    (model file added in the CRF milestone)
  tests/
    corpus/                   # labeled addresses and golden outputs (JSONL)
  tools/
    train/                    # Python training scripts (CRF milestone)
  benches/
  .github/workflows/
    ci.yml
    wheels.yml
```

---

## 5. Core design (`addr-core`)

### Pipeline

```
raw input
  -> clean/normalize (whitespace, case folding, punctuation, unicode)
  -> tokenize (with original spans)
  -> classify tokens against lexicons (suffix, directional, unit, state, number, ZIP, ordinal, fraction)
  -> tag (rules; CRF fallback when enabled and rules are low-confidence)
  -> assemble ParsedAddress + diagnostics
  -> emitters (read-only consumers of the struct)
```

Emitters (`usps`, `reso`, `key`, `slug`) never mutate the parsed struct and never re-parse.

### Principles

- **Deterministic by default.** Same input, same output, across platforms.
- **Spans preserved.** Components keep their source byte ranges so callers can highlight or debug.
- **Never panic on input.** Fuzz the tokenizer and assembler.
- **Lexicon access behind one interface** (a `Lexicons` struct built from `data/*.toml` at build time). This is the seam later state packs will plug into.
- No global mutable state; parser objects are cheap, `Send + Sync`.

### `ParsedAddress` (sketch)

```rust
pub struct ParsedAddress {
    pub street_number: Option<String>,        // "123", "123-125", "123 1/2"
    pub street_dir_prefix: Option<Directional>,
    pub street_name: Option<String>,
    pub street_suffix: Option<String>,        // canonical abbreviation
    pub street_dir_suffix: Option<Directional>,
    pub unit_type: Option<UnitType>,          // APT, UNIT, STE, ...
    pub unit_number: Option<String>,
    pub po_box: Option<String>,
    pub rural_route: Option<RuralRoute>,      // RR/HC + box
    pub city: Option<String>,
    pub state: Option<StateCode>,
    pub zip5: Option<String>,
    pub zip4: Option<String>,
    pub diagnostics: Diagnostics,
    pub source: SourceInfo,                   // original input, spans, input kind
}

pub struct Diagnostics {
    pub warnings: Vec<Warning>,               // enum: UnparsedRemainder, AmbiguousToken, MissingZip, ...
    pub unparsed: Option<String>,
    pub confidence: Option<f32>,              // present when CRF used; define semantics before exposing
    pub library_version: &'static str,
    pub model_version: Option<&'static str>,
}
```

Field names above are a starting point; keep struct naming aligned with the RESO emitter to reduce translation bugs.

### Public API (sketch)

```rust
let parser = Parser::new();                    // default options
let a = parser.parse("123 north main street apt 4, Miami FL 33101");
let b = parser.parse_fields(Fields { line1, line2, city, state, zip });

a.to_usps();        // UspsAddress { line1, line2, city, state, zip }; to_usps_wrapped(max_line)
a.to_reso();        // ResoAddress with RESO Data Dictionary field names
a.key(KeyOptions { include_unit: true });   // "k1:..." stable match key
a.slug();           // URL-safe, human-readable
```

`Parser::parse_many(&[&str])` for batch use, parallelized with rayon behind a feature.

---

## 6. Emitters and identifiers

### 6.1 USPS emit (Pub 28 style)

Rules to implement and test, each backed by a sourced table in `docs/usps-pub28-notes.md`:

- Uppercase, minimal punctuation.
- Standard street suffix abbreviations; directional abbreviations (N, S, E, W, NE, ...).
- Secondary unit designators (APT, STE, UNIT, ...) with line 1 / line 2 split.
- PO BOX, rural route / highway contract handling.
- ZIP+4 formatted with a hyphen.

**Verify every rule against the Pub 28 text itself** while implementing; do not rely on memory.

### 6.2 RESO emit

Map to RESO Data Dictionary address fields: `StreetNumber`, `StreetDirPrefix`, `StreetName`, `StreetSuffix`, `StreetDirSuffix`, `UnitNumber`, `City`, `StateOrProvince`, `PostalCode`, `PostalCodePlus4`. Check the current Data Dictionary version for additional fields (e.g. suffix modifier, additional street info) and record the version in `docs/reso-mapping.md`. The author has RESO experience and prior RESO Rust tooling, so the mapping was reviewed and accepted on 2026-10-15 (`docs/reso-mapping.md`, "Decisions").

### 6.3 Canonical key and slug

Two distinct outputs:

- **Key**: deterministic string from normalized components, for dedupe and joins. "123 N. Main Street, Apt 4" and "123 north main st #4" must produce the same key. Format starts with a version prefix (`k1:`). Changing the format later breaks stored values, so a format change means a new prefix, never an in-place edit.
- **Slug**: URL-safe, lowercase, hyphenated, human-readable. Display-oriented; not guaranteed unique.

Unit handling: `KeyOptions { include_unit: bool }`, because property-level and unit-level matching are different needs. Make it explicit in the type, with no hidden default surprises; document the default.

### 6.4 Key stability *(confirmed)*

City names are noisy and likely to be the field most often corrected by later tuning, so building the key from street components + ZIP5 + state (excluding city) keeps stored keys stable when city handling improves. Trade-off: addresses with missing ZIP get weaker keys; the key records that with the `k1n:` prefix and a diagnostics warning fires (`docs/key-format.md`). The slug is display-only and does include the city.

---

## 7. Python wrapper (`addr-py`)

- PyO3 with maturin; **abi3 wheels** so one wheel per platform covers many Python versions. Pick the minimum Python version (decided: **3.10**).
- Typed Rust-backed objects exposing the same fields, plus `to_dict()`. Ship type stubs (`.pyi`) and `py.typed`.
- `parse(str)`, `parse_fields(...)`, and **`parse_many(list[str])`** which releases the GIL during Rust work.
- Errors map to Python exceptions only for unusable input.
- No required runtime dependencies. No pandas/Polars dependency; document a short recipe instead.
- Optional usaddress-compatible tag output (`to_usaddress_tags()`), to ease migration.

---

## 8. CLI (`addr-cli`)

Small and optional, but useful from day one for debugging and bulk work:

- `addr parse "<address>"` prints JSON with diagnostics.
- `addr parse-file in.csv --column address -o out.csv` batch mode.
- `addr corpus test` runs the golden corpus and prints a diff summary (this doubles as the accuracy harness).

---

## 9. Testing and quality

The labeled corpus is the most important asset in the project. Build it first.
The format, runner and first cases are in place (`tests/corpus/README.md`); the runner fails on regressions against `tests/corpus/passing.txt`, not on cases the parser does not handle yet.

- **Golden corpus** in `tests/corpus/*.jsonl`. Each record: input, expected components, expected USPS output, expected RESO output, expected key. Seed it from the author's existing Florida code and test cases, plus public usaddress training data (MIT; keep attribution).
- **Property tests** (proptest): parsing the USPS output of a parsed address yields the same key; key equality across known variants; tokenizer never panics.
- **Fuzzing** for tokenizer and assembler (cargo-fuzz), added after the first stable milestone.
- **Benchmarks** (criterion) for single and batch parse; later a comparison script against Python usaddress on the same corpus.
- **Accuracy report** from `addr corpus test`: per-field accuracy and whole-address accuracy, tracked in CI output so regressions are visible.
- **Python tests** (pytest) covering the wrapper, type stubs, and `parse_many`.
- **CI**: `cargo fmt --check`, `clippy -D warnings`, `cargo test --all-features`, build wheels with maturin on Linux/macOS/Windows.

---

## 10. Initial files to build

Build in this order. Each milestone should end with passing tests.

Progress (2026-10-14): Milestones 0, 1 and 2 are complete. Milestone 4 items 19 and 20 are partly done (corpus: 159 cases, all passing; runner compares components, warnings, keys and USPS output). Milestone 3 is complete (item 17: `emit/key.rs`, `emit/usps.rs`, `emit/reso.rs`, `emit/slug.rs`, `docs/key-format.md`; the USPS layout rules were checked against the live Pub 28 text in October 2026, see `docs/usps-pub28-notes.md`); everything else is not started. `addr-cli` is a skeleton with only `parse`. The corpus is hand-labeled and the parser was tuned against it, so 100% says little about unseen data; see the Risks section.

### Milestone 0: scaffolding (done)

1. `Cargo.toml` (workspace), `rust-toolchain.toml`, `.gitignore`, `LICENSE-MIT`, `LICENSE-APACHE`
2. `README.md` (with the non-goals and the not-a-validator statement)
3. `CLAUDE.md` (from appendix A)
4. `crates/addr-core/Cargo.toml`, `crates/addr-core/src/lib.rs`
5. `.github/workflows/ci.yml`

### Milestone 1: data and types (done)

6. `data/suffixes.toml`, `directionals.toml`, `units.toml`, `states.toml`, `special.toml`, each with a source comment
7. `crates/addr-core/build.rs` (compile TOML into static tables) or a `include_str!` plus lazy parse if build script is overkill
8. `crates/addr-core/src/types.rs` (`ParsedAddress`, enums, `Diagnostics`)
9. `crates/addr-core/src/lexicon.rs`

### Milestone 2: parser (done)

10. `src/clean.rs` (input normalization)
11. `src/tokenize.rs` (tokens with spans)
12. `src/classify.rs` (token classification against lexicons)
13. `src/tagger/mod.rs`, `src/tagger/rules.rs`
14. `src/assemble.rs` (tags -> `ParsedAddress` + warnings)
15. `src/input.rs` (`Fields` multi-field input)
16. `src/parser.rs` (`Parser`, `parse`, `parse_fields`, `parse_many`)

### Milestone 3: emitters

17. `src/emit/mod.rs`, `emit/usps.rs`, `emit/reso.rs`, `emit/key.rs`, `emit/slug.rs`
18. `docs/usps-pub28-notes.md`, `docs/reso-mapping.md`, `docs/key-format.md`

### Milestone 4: corpus and CLI

19. `tests/corpus/*.jsonl` (seed from existing Florida code). *Done: auction-tracker test cases (`auction-tracker.jsonl`) and the raw lines of its golden CSVs (`auction-tracker-golden.jsonl`). Cases that need a place list wait for the Florida pack; the author's own Florida code is still pending (section 13, decision 1).*
20. `crates/addr-core/tests/corpus.rs` (golden test runner). *Done. Emitter fields (key, key_unit, slug, usps, reso) are on 170 of the 193 cases, written by `tools/corpus/emit_expect.py` from the specs, not from emitter output; the emitters matched on the first run.*
21. `crates/addr-cli/Cargo.toml`, `src/main.rs`. *Done: `addr parse` (JSON, `--pretty`, `--usps`, field options, stdin lines) and `addr parse-file` (CSV in, CSV out with `addr_`-prefixed columns). The CLI, not the core, depends on `csv` and `serde_json`. `addr corpus test [--dir DIR] [--verbose] [--update-baseline]` is done: the runner is the `addr_core::corpus` module behind the `corpus` feature (optional serde and serde_json), shared by `tests/corpus.rs` and the CLI. Exit codes: 0 ok, 2 malformed corpus, 3 regression or stale baseline.*
22. `benches/parse.rs`. *Done: criterion benches for single-address shapes, the whole corpus (`parse` and `parse_many`) and the emitters. First numbers (quick run, 2026-10-15): about 4 to 11 µs per address, about 110K addresses/s one by one, 107K/s through `parse_many` without `rayon` and about 3x that with it. Run `cargo bench -p addr-core --bench parse` (add `--features rayon`).*

### Milestone 5: Python wrapper

23. `crates/addr-py/Cargo.toml`, `pyproject.toml` (maturin), `src/lib.rs`
24. `python/addr/__init__.py`, `__init__.pyi`, `py.typed`
25. `crates/addr-py/tests/test_basic.py`, `test_batch.py`
26. `.github/workflows/wheels.yml`

### Milestone 6: CRF fallback (optional feature)

27. `tools/train/` (Python scripts: export labeled data, train with parserator/python-crfsuite, write model file)
28. `models/` (model file + `README.md` with provenance, version, training date)
29. `crates/addr-core/src/tagger/crf.rs` using `crfs`, behind feature `crf`; embed model with `include_bytes!`
30. Verify that a CRFsuite-format model trained from usaddress data loads in `crfs`; if not, pick a compatible export path before investing further.

### Milestone 7: hardening and release

31. Fuzz targets, usaddress comparison benchmark script, `CHANGELOG.md`, crates.io and PyPI publishing workflows
32. Re-check every lexicon table and the USPS emitter rules against the live Pub 28 pages (`docs/usps-pub28-notes.md`, "Verification status" and item 9 of the emitter verification list)

---

## 11. Deferred feature: state tuning packs

Not part of the initial build. Recorded here so the core is designed with it in mind.

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

Layering: base defaults, then state pack, then optional user pack from a directory; later layers win. Official packs could be embedded via cargo features (`state-fl`) and Python extras; any valid pack directory could be loaded at runtime. State selection would be two-pass: find state (state token or ZIP prefix), then tag with that state's pack; with no pack available, fall back to base only.

Likely Florida content: multiword and saint-prefixed places, ZIP-to-place validation to separate city from street name at the end of a line, and road naming conventions (SR/CR/US route variants, A1A, grid-style numbering in some counties).

Open questions to settle when this is picked up:

- Overlay semantics (add only, or add/replace/remove).
- Pack versioning and core compatibility checks (include pack version in output).
- ZIP data source and licensing: Census ZCTAs are public but not identical to USPS ZIPs; confirm redistribution terms for any USPS-derived table.
- Whether packs may ship their own model weights (suggest no initially; reserve a manifest field).
- A required golden-test corpus per pack, enforced in CI (`addr pack test`).

**Design-now constraints so packs can be added later:** keep all lexicon access behind the `Lexicons` interface; keep the key independent of city (6.4); keep diagnostics extensible; avoid hard-coding any state-specific rule in the tagger.

---

## 12. Risks

- **Accuracy parity with usaddress** is the main risk. Measure early on the Florida corpus; do not assume rules alone will match a trained CRF. Status: the rule parser passes all 134 hand-labeled corpus cases, but those cases shaped the rules. No held-out data or usaddress comparison exists yet; add both (a held-out set from usaddress's training data, then the comparison script) before claiming parity or deciding on Milestone 6.
- **Model format compatibility** between CRFsuite-trained models and the `crfs` crate must be verified before Milestone 6 work.
- **Key stability.** The key format is a public contract once stored; get it reviewed before release.
- **Licensing.** Confirm the data provenance of every lexicon table before publishing, and keep MIT attribution for any usaddress-derived training data.
- **Missing safety nets.** The never-panic rule and span validity are now covered by proptest properties (`crates/addr-core/tests/no_panic.rs`: arbitrary and address-shaped strings, multi-field input, every emitter, `parse_many` consistency). Fuzz targets and benchmarks do not exist yet (Milestones 4 and 7).
- **Python boundary overhead** for single-address calls; recommend `parse_many` in docs.

---

## 13. Open items for the author

0. **Pick-up point (end of session 1).** Done: Milestones 0 to 2, the key emitter, the USPS emitter. Done since: RESO emitter and `docs/reso-mapping.md` against Data Dictionary 2.0 (reviewed and accepted by the author, 2026-10-15). Done since: the slug (`emit/slug.rs`). Done since: proptest no-panic/span-validity property (`tests/no_panic.rs`), which closes Milestone 3. Done since: Milestone 4 (corpus expansion, `addr` CLI with `parse`, `parse-file` and `corpus test`, criterion benches, emitter expectations in the corpus). Next: Milestone 5 (Python wrapper). The Pub 28 verification list in `docs/usps-pub28-notes.md` is done except item 9, the release re-check of every lexicon table, which belongs to Milestone 7.

Decisions recorded 2026-10-15:

1. **Florida parsing code: pending.** The author will provide it later, as a reference (not ported). It seeds the corpus when it arrives; until then the corpus is the auction-tracker cases, `pub28.jsonl` and the emitter cases.
2. **Names: `us-address-parser`.** Publish names (crates.io and PyPI; Python import `us_address_parser`) will be `us-address-parser`; the CLI binary stays `addr`. The rename applies at Milestone 7. Until then the workspace crates keep the `addr-core`, `addr-cli` and `addr-py` names. Availability on crates.io and PyPI is not yet checked.
3. **Licensing: confirmed.** MIT OR Apache-2.0.
4. **Key definition: signed off.** `k1` (6.3, 6.4) is the public contract; any change needs a new version prefix. **Minimum Python version: 3.10** (abi3 wheels).
5. **RESO mapping: reviewed.** All seven choices were accepted as implemented; see "Decisions" in `docs/reso-mapping.md`.
6. **Dual addresses (Pub 28 section 215): hold both.** A record with a street address and a PO Box keeps both in `ParsedAddress` (street fields and `po_box`, warning `dual_address`). The key and the RESO output use the street; the USPS output writes the PO Box alone, as section 215 prefers. Rejected: keeping one and leaving the other unparsed, and returning two addresses (an API change for every caller).
7. **Attention lines: a field.** `ParsedAddress::attention` holds an ATTN or C/O line (section 214). No emitter writes it.

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

---

## Appendix A: Draft `CLAUDE.md`

```markdown
# CLAUDE.md

Rust-core US address parser with a PyO3 Python wrapper. Read docs/design.md first.

## Rules
- addr-core must stay pure Rust with no Python dependency. PyO3 lives only in addr-py.
- Never panic on input. Parsing always returns a ParsedAddress with diagnostics.
- Emitters read ParsedAddress; they never re-parse or mutate it.
- All lexicon access goes through the Lexicons interface. No state-specific logic in the tagger.
- The canonical key format is a public contract. Any change requires a new version prefix and a docs/key-format.md update.
- Every USPS or RESO rule implemented must be traceable to a source in docs/.
- This library formats and parses; it does not validate deliverability. Do not imply otherwise in docs or code comments.

## Workflow
- Run `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test --all-features` before finishing a task.
- Add a golden-corpus case for every parsing bug fixed.
- Prefer small commits aligned with the milestones in docs/design.md.
- Python: `maturin develop` then `pytest crates/addr-py/tests`.
```
