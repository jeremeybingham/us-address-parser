# Changelog

Nothing is released yet. Entries are newest first.

## Unreleased

### Dataframe helpers (item 50)

- `us_address_parser.pandas.parse_column(series, ...)` and
  `us_address_parser.polars.parse_column(series, ...)` parse a column of
  one-line addresses with `parse_many` and return a DataFrame of component
  columns (street parts, unit, city, state, ZIP, address type, key, slug,
  warnings; `columns=`, `prefix=` and `default_state=` choose others). Rows
  with no address text are null. pandas or Polars is imported when the
  function is called; install the extras `[pandas]` or `[polars]`.
- Custom layer (items 47 to 49) and its guide (60) deferred past the first
  release (decision 45).

### The last two Florida sample errors (item 36)

- A two-letter directional written as two words (`N W 27 AVE`,
  `MAIN ST S W`) is NW, NE, SW or SE.
- A second house number before a street (`1309 1329 E VINE ST`) is left
  unparsed with `multiple_addresses`, only when a name and the suffix follow
  (rural grid streets such as `1575 NORTH AVE` are unchanged).
- Box words typed before a numbered street (`P O BOX 8491 N W 17TH ST`) are
  left unparsed and the street is the address; `PO BOX 12 N MAIN ST` stays a
  box.
- Corpus `rg-137` to `rg-144`; the Florida sample is 187 of 190 (the other
  three are bad source data). Keys change where the old parse was wrong
  (`docs/key-format.md`): 4,313 property keys and 4,313 unit-level keys of
  961,275 distinct Florida licensee addresses. The blind geocode benchmark is
  unchanged at 96.4%.

### Confidence score deferred (item 38, decision 43)

- `Diagnostics::confidence` stays `None` in the first release: the labeled
  data supports only a coarse score. The field is kept (reserved) so that
  adding the score later changes no signature.
- `tools/calibrate/fit.py` fits a sign-constrained logistic model on the
  corpus, usaddress's test files and the Florida sample (exploratory; nothing
  it writes ships). `addr corpus features` and `addr parse --features` print
  the tagger's decisions and the warnings per case for it. No parse or output
  changes.

### Parser fixes from the Florida sample (item 36)

- A highway route may start with a letter after a highway prefix
  (`HIGHWAY A1A`); a lone number on the next line after a highway is a unit.
- `178ST` is split into `178 ST` when `ST` (or `RD`) is not the number's
  ordinal ending; `21ST` stays whole.
- `DR E 203` is postdirectional E and unit 203; `E 203` on its own line is
  still the unit.
- `2ND FLOOR #260` is floor 2 and unit 260; `PH FLOOR` is the unit PH.
- A copy of the city after a unit (`UNIT 412 INLET BEACH`), or repeating the
  city's last words in its own segment (`RD,CLOUD`), is dropped.
- A period after an abbreviation separates words (`ST.CLOUD` is `ST CLOUD`)
  unless the pieces form a known word (`BLV.D`); `BOX.2351` is split like
  `APT.1285`.
- Corpus `rg-121` to `rg-136`. Keys change where the old parse was wrong
  (`docs/key-format.md`): 1,691 property keys and 1,865 unit-level keys of
  961,275 distinct Florida licensee addresses.

### Collaborative labeling for the Florida sample (item 36)

- `tools/calibrate/triage.py` checks each unlabeled draft without trusting
  the parser (every input word accounted for, in address order; a known
  city; no warnings; nothing that needs judgment), accepts checked drafts in
  bulk (`by=assistant` in the note), and applies approved recommendations
  (`by=assistant+author`), adding their parser errors to `NOTES.md`.
- `tools/calibrate/README.md` documents the tools and the collaborative
  mode, so labeling can stop and restart at any step.

### Release preparation planned (decision 42)

- New plan items before the first release: benchmarks (57), choosing a new
  name to replace decision 33's (58), the text cleaning (61) and a fresh
  repository with no history (62), with publishing (46) from the new one.
  Milestone 9 gains examples (59) and a customization guide (60), and the
  custom layer's scope widens to every table the tagger reads.
- `docs/repository_cleaning.md` collects every deferred text change in one
  checklist: build-process wording, naming the private data sources,
  attributions, a user-focused README with diagrams, a neutral tone toward
  usaddress, old names, and the fresh-repository steps. `CLAUDE.md` points
  new cleanup items there.

### Blind Florida sample drawn (Milestone 7, item 36)

- `tools/calibrate/sample_fl.py` draws the sample for calibrated confidence
  from the private Florida files (auction listings and four DBPR exports,
  address columns only): 150 rows uniformly at random over distinct
  addresses with a fixed seed, plus a 40-row stratum the parser flags,
  skipping inputs already in the corpus. It writes corpus-format records
  pre-filled with the current parse for the author to correct; `--status`
  counts what is left. The rows stay in `local-data/`.
- `tools/calibrate/label.py` labels the sample row by row in the terminal:
  the draft is hidden until asked for, suffixes, directionals, unit types and
  states are picked from the standard forms, the warnings that follow from
  the fields are computed, every saved row passes the corpus runner's
  checks, and rows that differ from the parser get a line in `NOTES.md`.

### The tagger records its uncertain decisions (Milestone 7, item 37)

- Internal only: the tagger now notes which uncertain rules settled a parse
  (the city by comma, by the suffix split or by the place list; a state with
  no ZIP; a unit with no designator; a letter joined to the house number;
  the `default_state` hint). The confidence score (item 38) will be fitted on
  them. No output or key changes.

### Release order (decision 41)

- Milestone 7 (calibrated confidence) now runs before the rest of Milestone 8
  (the Pub 28 recheck, the rename and packaging, publishing): the hand-labeled
  Florida sample may find key-changing bugs, which must land before `k1`
  freezes. Order: items 37, 36, 38, 44, 45, 46.

### Netsyms comparison and eight parser fixes (item 56, decision 40)

- `tools/netsyms/` samples, profiles and scores the parser against the Netsyms
  Address Database (private data, kept in `local-data/`). On 217,906 uniform
  US rows (213,551 without malformed source fields) the whole address matches
  the source on all six fields in 99.81% to 99.86% of rows, up from 99.20% to
  99.54%. Blind geocode benchmark: 96.1% to 96.4% whole-address.
- Fixes (corpus `ns-001` to `ns-023`, key changes in `docs/key-format.md`):
  a suffix glued to a city is split only with a real anchor (`PICO RIVERA`,
  `CREEKWOOD`, `AVENIDA DE LA LUNA`, `STRAND` stay whole); a word after `#`
  and a letter range are units; values after `#` or a designator are not
  split (`# 2WD`, `APT I4`); `U S ROUTE 23` keeps its house number; a
  postdirectional is no longer dropped as a cut-off copy of a city starting
  with NE; `CT`, `LA` and `WY` are states on one line when the ZIP and a place
  say so; `WASHINGTON DC` keeps its city.

### Outside sources deferred (decision 39)

- `docs/future-sources.md`: NAD, OpenAddresses, the Netsyms Address Database
  and a Kaggle address list evaluated (terms, fields, what each could test),
  what the v1 accuracy numbers cover and do not, and a v2 plan. Nothing was
  downloaded or parsed. Design.md decision 39 defers source-driven parser
  work to the `k2` version.

### Fuzz targets (item 43)

- `fuzz/` (cargo-fuzz, nightly, outside the workspace): `parse` and
  `parse_fields` run the public parse paths and every emitter. A CI job runs
  each for a short time.

### Intersections (item 55)

- `MAIN ST & 1ST AVE` (also `AND`, `@`, `AT`) is detected when a name and a
  street suffix come before the connector and a street suffix after it. The
  first street is parsed, the second stays in `unparsed`, the new warning
  `intersection_detected` is raised (`Warning::ALL` grows to 22), and `key()`
  and `slug()` are `None`. `LAND AND SEA RD`, `BEACH AT SEA DR`,
  `STATE AND MAIN ST` and `AT&T WAY` are single streets and do not warn.
- Corpus `is-001` to `is-013`; the USPS round-trip check skips intersections;
  key-format log entry.

### Trailing country (item 54)

- A trailing `USA`, `US`, `UNITED STATES` or `UNITED STATES OF AMERICA` is
  dropped before tagging and raises the new warning `country_ignored`
  (`Warning::ALL` grows to 21). It is cut after a comma, a ZIP or a state, or
  as a whole field other than the street line; `US` in a highway name or
  after a street stays. Other countries are not recognized. The names are in
  `data/special.toml` `[country]` (`Lexicons::is_country`).
- Corpus `co-001` to `co-015` (`country.jsonl`); key-format log entry.

### Removed unused surface (item 53)

- `Diagnostics::model_version` (and the Python `model_version` property and
  stub entry) and the empty `crf` cargo feature. No parse or key changes.

### Documentation review after Milestone 6

- Status, plan and layout brought up to date in `docs/design.md` and the
  README: Milestones 0 to 6 done (353 corpus cases, 27 Python tests, wheels),
  the Python package documented, completed items condensed, and Milestone 8
  re-planned with items 53 to 55 first.
- New decisions 33 to 38 (design.md): the Rust crates will be published as
  `us-addr-core` and `us-addr-cli` because `us-address-parser` is taken on
  crates.io (PyPI keeps `us-address-parser`); `k1` freezes at the first
  release, with a dated change log in `docs/key-format.md`; the unused
  `model_version` and `crf` go (item 53); a trailing country and intersections
  get warnings only (items 54 and 55); a non-place city keeps `place = None`;
  Milestone 8 runs before Milestone 7.
- `docs/usps-pub28-notes.md` lists the source of every rule added since
  2026-10-16; `docs/usaddress-comparison.md`, `docs/packs/fl.md` and
  `tests/corpus/README.md` are refreshed; `CLAUDE.md` states the key policy and
  the current commands. The `models/` placeholder (for the dropped CRF) is
  removed.

### Wheels for Linux, macOS and Windows (Milestone 6, item 35)

- `.github/workflows/wheels.yml` builds abi3 wheels with maturin on every pull
  request and push to `main`: Linux x86_64 and aarch64 (manylinux), macOS
  (universal2, one file for both architectures) and Windows x64, uploaded as
  artifacts. Each wheel is then installed (not the source tree) on Python
  3.10 and 3.13 and the pytest suite runs, which shows one abi3 file serving
  both ends of the supported range. Nothing is published (item 46).
- Known gap for publishing: an sdist does not carry the workspace's `data/`
  directory, so building from it fails; the tables have to move into the
  package first.

### Polars in the batch recipe (Milestone 6, item 34 follow-up)

- The Python README now has a tested Polars recipe next to the pandas one:
  chosen columns with an explicit `pl.String` schema (an all-`None` column
  stays typed), and `to_dict()` structs read with `struct.field`. Building a
  7-column frame from 100,000 addresses: 0.34 s with Polars, 0.45 s with
  pandas, parsing included. `bench_batch.py` times both; the CI Python job
  installs polars.

### Python `parse_many`: batch tests, benchmark and the pandas recipe (Milestone 6, item 34)

- `crates/addr-py/tests/test_batch.py`: `parse_many` gives what a loop of
  `parse` gives, keeps order, returns `None` for unusable input, takes
  `default_state`, accepts a list or tuple (a string or generator raises
  `TypeError`), and lets another thread run during a big batch (the GIL is
  released). Both pandas recipes are tested.
- `crates/addr-py/benchmarks/bench_batch.py`: on 100,000 corpus addresses
  (14 CPUs) `parse_many` takes 0.25 s against 1.55 s for a loop of `parse`
  (6.3x); `to_dict` per row costs about as much as parsing, so the README
  recipe picks columns, with `json_normalize(to_dict())` for everything.
- The CI Python job installs pandas so the recipe tests run.

### The Python wrapper (Milestone 6, item 33)

- `crates/addr-py` joins the workspace: the extension module
  `us_address_parser._core` (PyO3 0.30, abi3 for Python 3.10 and later, built
  with maturin) and the `us_address_parser` package with type stubs and
  `py.typed`.
- `parse`, `parse_fields`, `parse_many` (the GIL is released and the batch
  parses in parallel), `tag` and `parse_tokens` (usaddress's shapes),
  `address_type`, and a `Parser(default_state=...)`. A read-only
  `ParsedAddress` with the components, `place`, `warnings`, `key()`,
  `slug()`, `to_usps()`, `to_reso()`, `to_usaddress_tags()`, `tag()`,
  `repeated_labels()` and `to_dict()`. `ParseError` (a `ValueError`) only for
  input with no address text; a `default_state` that is not a state raises
  `ValueError`.
- 17 pytest tests (`crates/addr-py/tests`), including `tag` against usaddress
  when it is installed, and a CI job (Python 3.10 and 3.13). Plain `cargo
  test` and `clippy` need no libpython (the crate has no Rust test target).

### `tag()` and `address_type()` (Milestone 6, item 31, commit 1)

- `ParsedAddress::tag()` is the counterpart of `usaddress.tag()`: tokens that
  share a usaddress label are joined with a space in input order, with the
  ends of each value stripped of spaces, commas and semicolons. Unlike
  usaddress it never fails: a label in two separate runs (two units) is
  joined and reported (`TaggedAddress::repeated_labels`, new
  `Warning::RepeatedLabel` / `repeated_label`, which `tag()` reports and
  `Diagnostics` never carries). Tokens the parse did not place are left out.
- `ParsedAddress::address_type()`: `Street`, `PoBox`, `RuralRoute`,
  `Military`, `GeneralDelivery`, `Dual` or `Unknown`.
  `TaggedAddress::usaddress_type()` gives usaddress's `Street Address`, `PO
  Box` or `Ambiguous` by its rule, so a dual address is a Street Address and
  a rural route a PO Box, as there.
- `addr parse --tag`; corpus fields `address_type`, `tag`, `tag_repeated`;
  corpus `tg-001` to `tg-008`.
- Migration notes (item 32): "Coming from usaddress" in the README has the
  call mapping and the differences; `crates/addr-py/README.md` has the
  planned Python names.
- `tools/compare/compare_usaddress.py ... tag` compares the joined dicts and
  types with `usaddress.tag()` on the vendored test files: identical on 99.3%
  of 8,103 addresses, types on 99.8% (`docs/usaddress-comparison.md`).

### `default_state`: a state hint for lookups (Milestone 6, item 30)

- `ParseOptions { default_state }` and `Parser::with_options` (fails with
  `OptionsError::UnknownDefaultState` for anything that is not a US state,
  DC or territory), `addr parse` and `parse-file --default-state ST`, and a
  `default_state` field on corpus cases.
- When the input names no state and has no known ZIP, a city is looked up in
  the hinted state (`281 EVELYN DR DOTHAN` with `AL` finds DOTHAN, which the
  blind benchmark's largest remaining error class needed), and the state's
  pack readings apply (`CR` is CIRCLE and `SR` is STATE ROAD with `FL`).
- It never fills `state`: `missing_state` still fires and the key keeps its
  `k1n` form. New warning `default_state_applied` (`Warning::ALL` is 19) when
  the hint decided a place match or a reading. The input's own state, or a
  known ZIP, always wins and the hint is then ignored.
- Corpus `ds-001` to `ds-007`. On the 9,314 Florida auction rows with the
  state and ZIP removed, `--default-state FL` finds a place for 8,828 rows
  (1,273 without the hint); the components of only 72 rows change (a lone
  city word becomes a city, and `155 OCEAN LANE DR 1007, KEY BISCAYNE` finds
  its comma city).

### Florida pack: communities, spellings and ZIP pairs from five DBPR exports

- `tools/packs/dbpr.py` reads the DBPR parquet and board CSV exports;
  `observe_fl_zips.py` and `measure_fl_places.py` take any number of them and
  count each address once (961,275 distinct Florida addresses in five files).
- `places_added.tsv` gains 17 communities (`MIDDLETON`, `CUTLER RIDGE`,
  `PEMBROKE LAKES`, `CAROL CITY`, `DUETTE`, `DEER ISLAND`, ...), 36 in all;
  `variants_added.tsv` gains 21 spellings (`JAX`, `PBG`, `STPETE`, `OVEIDO`,
  `LEHIGH ACERS`, ...), 37 in all; `zips_observed.tsv` is 341 pairs (was
  239). Pack version 2026.10.4.
- The observed-ZIP rule now also needs 10-address pairs to be 1% of the
  ZIP's addresses, so a big neighbour city written for a suburb stays a
  mismatch as the data grows.
- On the three newer files `zip_city_mismatch` fell from 5,509 to 4,505 and
  Florida rows with an unknown city from 3,289 to 2,776. Corpus `fl-028` to
  `fl-033`.

### Parser fixes found in three more DBPR licensee files

Parsing the cosmetology, construction and `lic03bb` licensee files (613,719
distinct addresses, 93% Florida; `docs/packs/fl.md`) found four parser
problems. 4,225 of those addresses now come out differently, and none of the
other benchmarks moved (blind geocode 96.1%, usaddress files 99.2%).

- **A cut-off copy of the address ahead of the whole one** (`624 E Coloni 624
  E COLONIAL DR`; 1,785 addresses): the copy is dropped instead of polluting
  the street name. The house number comes again after words that match the
  start of what follows, the last one possibly cut short. Corpus `rg-115` to
  `rg-118`.
- **A designator joined to its value** (`APT.1285`, `STE.200`, `APT:201`,
  `UNIT-4`, `APT3D`; about 1,200 addresses): was `# APT1285` or unparsed, now
  `APT 1285`. Tokenizer rule. Corpus `rg-106` to `rg-110`.
- **A name or building on line 1, the street on line 2** (`THE COVE` /
  `1111 ASBURY WAY`; 629 addresses): line 1 is the attention line. Also in
  the first comma segment of one line. Corpus `rg-119`, `rg-120`.
- **Units with no Pub 28 word**: `BAY 102`, `STUDIO 7` and `BOOTH 12` are
  read as `UNIT` (observed spellings, with `non_standard_abbreviation`);
  `SUITE NO. 102` drops the `NO`. An observed spelling with no value after
  it (`GULF TO BAY`, `ARUBA BAY`) is no longer taken for a unit. Corpus
  `rg-111` to `rg-114`.
- Across the three files: `unparsed_remainder` 5,274 to 4,334,
  `missing_street_number` 2,018 to 1,383, `possibly_truncated` 1,168 to
  1,082; `non_standard_abbreviation` rose 2,589 to 3,056 (the new observed
  unit words).

### No CRF fallback (item 29, decision 32)

- The usaddress comparison and the blind geocode benchmark were rerun after
  the place list and the Florida work. Whole address (coarse) on the blind
  benchmark: ours 96.1%, usaddress 88.2%; on usaddress's own test files ours
  99.2%, usaddress 100.0% (its regression tests). No gap remains that a CRF
  would close, so none will be built: plan items 39 to 42 and 52 are dropped.
  Numbers in `docs/usaddress-comparison.md`; decision in `docs/design.md`.

### Florida: communities and spellings still missing after the national list (item 28)

- `tools/packs/measure_fl_places.py` lists the cities in the DBPR export that
  match no place. After the national list 2,957 rows (849 cities) did.
- The pack gains `places_added.tsv` (19 communities: `COCONUT GROVE`,
  `VIERA`, `HARMONY`, `CHAMPIONS GATE`, `UNIVERSITY PARK`, `NORTH VENICE`,
  `HUTCHINSON ISLAND`, `SINGER ISLAND`, ...) and `variants_added.tsv` (16
  spellings of known places: `PT CHARLOTTE`, `STPETE BEACH`, `LA BELLE`,
  ...), both hand-kept with the number of addresses behind each row and read
  add-only by `build.rs`. A variant of a place only the national list has is
  added after it. `zips_observed.tsv` is rerun with the new places (239
  pairs, was 218). Pack version 2026.10.3.
- Unmatched rows: 2,957 to 1,348. Corpus `fl-024` to `fl-027`; `fl-016` uses
  `LAUDERDALE BY THE S` now that `LAUDERDALE BY THE SE` is a variant.

### Rarer misspellings and the penthouse shape (item 27)

- Observed spellings `DRT` (DR), `LNE` (LANE) and `SU` (SUITE), from the
  DBPR export; they raise `non_standard_abbreviation`.
- A penthouse right after a designator is one unit: `APT PH 5213` is unit
  `PH5213`, `UNIT PH6` stays `PH6`, and `SUITE PH` is `STE PH` (decision
  31; 162 DBPR lines). It used to come out as a designator with no number
  plus a second PH unit and `missing_unit_number`. Corpus `rg-098` to
  `rg-105`.

### Florida: `SR` is STATE ROAD, `CTY HWY` is COUNTY HIGHWAY

- The Florida pack's `overrides.toml` can add highway prefix spellings
  (`[[highway]]`: `spelling`, `highway`, `kind`), read through the new
  `Lexicons::highway_in(state, spelling)`. Add-only: a spelling the base
  table has is an error. The tagger and the assembler pass the address's
  state (its own, or the ZIP's).
- `SR` before a route number is State Road in Florida (139 DBPR rows), a
  listed variant: `22200 SR 60` is highway `STATE ROAD 60`. Elsewhere it
  stays a street name. `CTY HWY` (5 rows) is COUNTY HIGHWAY, flagged
  non-standard. `CTY RD` has no evidence and is not added. Corpus `fl-020` to
  `fl-023`.
- The `k1` key of such an address changes (`SR 51` becomes `STATE ROAD 51`
  in the name segment). The key format is unchanged.

### The parser reads the national place list in every state

- `Lexicons::place`, `zip_places`, `places` and `is_place` cover all 52
  states; the place behaviors that worked only in Florida (a city on a line
  with no commas, a street versus a city before a ZIP, a city run into the
  street, a cut-off city completed from the ZIP, the place from the ZIP) now
  work everywhere. `ParsedAddress::place` is set in every state.
- `zip_city_mismatch` is still raised only in states with a curated pack
  (`Lexicons::has_pack`); documented on the warning (decision 30).
- Blind geocode benchmark, whole address (coarse): 85.7% to 96.1%
  (usaddress 88.2%); `PlaceName` tokens 87.6% to 96.8%. Corpus
  `pl-001` to `pl-008`; `fl-013` and `fl-019` now name their place. Parse
  time: corpus +6.5%, single addresses unchanged.

### National place list: compiled in (feature `places-us`)

- `build.rs` compiles `data/places/` under the state packs; `places-us` is on
  by default. The place tables are now packed binary records read with
  `include_bytes!` instead of `&str` tables: the national list adds 2.7 MB to
  the `addr` binary (13.4 MB as `&str` tables). `docs/places.md`.
- Florida gains 405 Census places its pack lacked (`TRINITY`, `CITRUS
  SPRINGS`, ...). The Census's `Everglades` is renamed `EVERGLADES CITY`, the
  pack's name. Lookups for other states stay off until the parser uses the
  list (item 25), so only Florida parses differently.
- `zips_observed.tsv` rerun with the new places: 218 pairs (was 198), among
  them `BELLVIEW` in 32526 (`KNOWN` in `observe_fl_zips.py`), so `rg-030`
  keeps no `zip_city_mismatch`. Pack version 2026.10.2.
- New `PlaceKind::Subdivision` (a township and the like).

### National place list: builder and data (`data/places/`)

- `tools/places/build_us.py` builds `data/places/places.tsv` (50,375 places
  in the 50 states, DC and Puerto Rico: Census places and active county
  subdivisions, plus GeoNames postal names) and `zips.tsv` (41,154 ZIPs).
  The cleaned names, the 87.4% city coverage on the blind benchmark, and
  what the builder settles (one county per place, the 462 newer places,
  Puerto Rico) are in `docs/places.md`. Data only: the parser does not read
  it yet (items 24 commit 2 and 25).

### Slug follows the street for dual addresses

- `slug()` of an address with a street and a PO Box now names the street and
  its units (`9418-cuba-rd-hayes-va-23072`) instead of the PO Box, so the
  slug, key and RESO output agree. The USPS output still writes the PO Box
  (Pub 28 section 215). The slug is not a contract, so the key is unchanged.
  Corpus `rg-040` to `rg-044`; `docs/key-format.md`.

### `addr parse-file`: every component has a column

- New columns `addr_second_unit_type`, `addr_second_unit_number` (after
  `addr_unit_number`), `addr_general_delivery` and
  `addr_military_post_office`. A second unit (`STE 201 PMB 308`) used to be
  left out of the CSV. A test checks that every component `addr parse`
  prints has a column.

### Florida pack: observed ZIP places and `CR` as CIRCLE

From the DBPR licensee export (decisions 20 and 21 of 2026-10-16 in
`docs/design.md`; `docs/packs/fl.md`; corpus `fl-014` to `fl-019`).

- `data/packs/fl/zips_observed.tsv`: 198 (ZIP, place) pairs seen in at least
  10 distinct addresses (or 3 that are 5% of a small ZIP), added to the ZIPs'
  shared places by `build.rs`. Built by `tools/packs/observe_fl_zips.py`.
  `zip_city_mismatch` on the export fell from 23,878 rows to 1,871, and 59
  more cut-off cities complete.
- Pack `overrides.toml` and `Lexicons::suffix_in`: a pack may read a base
  suffix spelling as another suffix. Florida reads `CR` as CIRCLE (CREEK
  elsewhere); 333 rows changed, keys included. Pack version 2026.10.1.
- `tools/corpus/emit_expect.py`: dual addresses key and map to RESO from the
  street and their USPS line and slug are the PO Box alone; name words after
  a highway route are kept. New `--check` mode, run in CI. 72 cases that
  had no emitter expectations now have them.

### Rule fixes from the DBPR licensee data

Run on about 445,000 Florida addresses from the DBPR real estate licensee
export; corpus cases `rg-082` to `rg-097`; rules and observed spellings in
`docs/usps-pub28-notes.md`. Rows with any `unparsed_remainder` fell from
4,414 to 2,787, and 320 rows that parsed wrongly with no warning (a unit
read into the street name) are now right. Keys changed for the fixed rows
(`docs/key-format.md`).

- A line 2 that is exactly the city field is skipped, even when it starts
  with a suffix word (`CAPE CORAL`, `FORT MYERS`).
- A unit value in two parts after a designator or `#` is one value
  (`APT E 1003`, `UNIT 210 C`).
- A street cut across lines is joined (`2781 NE` / `15 ST`).
- A dash between the street and its unit is dropped.
- Observed spellings: `APTO`, `AP`, `APART`, `APARTAMENTO`, `APARTAMENT`,
  `APPT`, `APP`, `APRT` (APARTMENT), `SUIT` (SUITE), `BLV`, `BL`
  (BOULEVARD), `TE` (TERRACE), with `non_standard_abbreviation`.

### Florida pack and place lookups (Milestone 5)

Decisions 13 to 19 of 2026-10-16 in `docs/design.md`; data and sources in
`docs/packs/fl.md`; corpus cases `fl-001` to `fl-013`.

- State packs: `data/packs/<st>/` compiled in by `build.rs` behind a
  `state-<st>` feature. The Florida pack (`state-fl`, on by default) has 665
  places, 998 variant spellings and 1,473 ZIPs, exported from
  auction-tracker's `cities_fl.json` by `tools/packs/build_fl.py`. GeoNames
  ZIP data is CC BY 4.0; see `NOTICE`.
- `Lexicons::place`, `zip_places`, `place_key`, `is_place`, `has_pack` and
  `places`; new types `Place`, `PlaceKind`, `PlaceMatch`, `PlaceSource`,
  `ZipPlaces`. Place names compare without case, punctuation, spacing or
  abbreviations (`W PALM BCH` is WEST PALM BEACH).
- New `ParsedAddress::place`: the place the city names, a listed
  misspelling, a cut-off city completed from the ZIP, or the ZIP's place
  when there is no city. `city` is unchanged. The CLI prints `place`;
  `parse-file` adds `addr_place` and `addr_place_source`.
- New `Warning::ZipCityMismatch` (`zip_city_mismatch`): a known city whose
  ZIP the pack does not list for it. `rg-041` now expects it.
- On one line, a place the pack knows sets where the city starts, before
  the suffix rule (`123 MAIN ST WEST PALM BEACH FL`); a street suffix run
  into a place is split (`WILSON STREETDADE CITY FL`); with a pack, a first
  segment ending in a suffix that is no place is the street
  (`WINGATE ST, 32507`).

### Rule changes from the usaddress comparison and the geocode benchmark

Corpus cases `rg-050` to `rg-081`; rules in `docs/usps-pub28-notes.md`;
decisions 8 and 9 of 2026-10-15 in `docs/design.md`. On the blind geocode
benchmark, whole-address accuracy (coarse) rose from 76.1% to 85.4%
(usaddress: 88.2%).

- A highway route ends the street when finding the city on one line. Name
  words after the route are kept in the USPS, RESO and key output
  (`HIGHWAY 66 FRONTAGE RD` lost FRONTAGE before), and words on a later line
  are unparsed instead of hidden.
- Street and city in the first comma segment are split; a unit starting the
  city's segment, or before the street, is a unit.
- A no-ZIP state at the end of one line; hyphenated routes (`GA-25`,
  `US-385`, `CR-48`); `BOX <number>` alone as a PO Box; military box line in
  reverse order; HC with ROUTE; `HGWY` and `COUNTRY ROAD` as observed
  spellings (the highway table takes `observed` forms).
- House numbers: a letter before a directional (`428 B SOUTH`), letter and
  digits (`T703`), two-part grid numbers (`N165 W2123`). Letter routes after
  a county prefix (`COUNTY ROAD J`). A stray backtick is trimmed.
- New `Warning::MissingStreetName` (`missing_street_name`) for a house number
  and a unit with no street.
- Ordinal floors are numbers (`5TH FLOOR` is FL 5), which changes unit-level
  keys for them.
- New `ParsedAddress::building` (`Component::Building`, usaddress
  `BuildingName`) from a `[building]` word list in `data/special.toml`
  (`Lexicons::is_building_word`). Not emitted; the CLI prints it and
  `parse-file` adds `addr_building`.
- `tests/usaddress_gold.rs` floors raised to 99.7% of tokens and 99.4% of
  addresses.

### usaddress comparison (design decision 11; first planned for Milestone 8)

- `ParsedAddress::to_usaddress_tags()` (`UsaddressTag`): each input token with
  its usaddress label, from the component spans. `addr parse --usaddress`
  prints them as JSON.
- `tests/external/usaddress/`: usaddress's labeled test files (MIT, pinned
  commit) and `in_training.txt` (`tools/compare/training_overlap.py`).
- `tools/compare/compare_usaddress.py`: runs both parsers and scores them by
  the same rules, on labeled files (`gold`) or raw CSV and text (`raw`).
- `crates/addr-core/tests/usaddress_gold.rs`: fails if this parser's coarse
  score on those files drops below the recorded floor.
- Results and the backlog: `docs/usaddress-comparison.md`.

### Dual addresses and attention lines

- Dual addresses (Pub 28 section 215; design decision 6 of 2026-10-15): a
  record with a street address and a PO Box keeps both, with the new
  `dual_address` warning. The key and `to_reso()` use the street; `to_usps()`
  (and so the slug) writes the PO Box alone. Corpus cases `rg-040` to
  `rg-045`.
- `ParsedAddress::attention` (section 214): ATTN, ATTENTION, C/O and CARE OF
  lines, from `data/special.toml` `[attention]`. Not emitted; the CLI prints
  it and `parse-file` adds `addr_attention`. Corpus cases `rg-046` to `rg-049`.

### Parser fixes from real Florida and out-of-state data

Corpus cases `rg-017` to `rg-039` (auction listings, DBPR license files):

- County grid house numbers (`22W410`, `W338S9739`, `S1749`).
- A postdirectional after a comma belongs to the street (`GOODWOOD BLVD., SE`).
- Floors: `3RD FLOOR`, and `5TH FLOOR` after a one-word street name.
- A line 2 that is only a number is a unit with no designator, when a street
  name comes first. A delivery line written twice is read once.
- A street suffix ending the line is not a cut-off designator (`DRY CREEK TRL`
  lost its `TRL`).
- The city-copy rule no longer drops a unit value or a single letter
  (`SUITE A` before `ARCHER`, `13TH AVE S` before `ST PETERSBURG`).
- PMB is a unit designator (Pub 28 section 285).
- `FL 40` at the start of the street is a state highway, not floor 40.
- `12 SIMON CT 32505`: CT is the COURT suffix when no city precedes it.
- Multi-field input drops a trailing copy of the state field from the address
  line, and a line 2 that repeats the end of line 1.
- A comma list of house numbers raises `multiple_addresses`.
- Asterisks are trimmed from words; `PRIVATE ADDRESS` is a placeholder.
- `Highway::name()`; the CLI's `highway` output includes the state code.

### Milestone 4: corpus, CLI and benchmarks

- `addr` CLI: `addr parse` prints one JSON record (components, warnings, key,
  slug, USPS and RESO output), with `--pretty`, `--usps`, field options, or
  one address per stdin line. `addr parse-file IN.csv --column NAME
  [-o OUT.csv]` appends `addr_`-prefixed columns. `addr corpus test [--dir
  DIR] [--verbose] [--update-baseline]`.
- Corpus runner moved into `addr_core::corpus` (feature `corpus`, optional
  serde and serde_json; the default build is still dependency-free).
- `benches/parse.rs`: criterion benchmarks for six address shapes, the whole
  corpus (`parse` and `parse_many`) and the four emitters.
- Corpus: `auction-tracker-golden.jsonl` from the raw address lines of
  auction-tracker's golden CSVs, hand-labeled. Emitter expectations (`key`,
  `key_unit`, `slug`, `usps`, `reso`) generated by
  `tools/corpus/emit_expect.py` from the specs, without reading the emitters.
- The tagger skips a ZIP ending the address line that repeats the ZIP on the
  last line. `LP` is an observed spelling of LOOP.

### Milestone 3: emitters

- Canonical key `k1` (`ParsedAddress::key(KeyOptions)`), spec in
  `docs/key-format.md`.
- USPS emitter (`to_usps()`, `to_usps_wrapped(max_line)`,
  `UspsAddress::lines()`), checked against the live Pub 28 text: the unit at
  the end of the delivery line (213), no fixed line length (35), a period
  between digits kept (`39.2 RD`, 232), a state inside a highway name written
  as its code (`KY HIGHWAY 1207`, Appendix F, `Highway::state`).
- RESO emitter (`to_reso()`, `ResoAddress`) for Data Dictionary 2.0;
  `fields()` and `length_overruns()`. Mapping and the author's decisions in
  `docs/reso-mapping.md`.
- Slug (`slug()`), display only.
- Proptest no-panic and span-validity properties (`tests/no_panic.rs`).
- Corpus: `keys.jsonl`, `reso.jsonl`, 17 more Pub 28 cases, and a test that
  parsing the USPS output gives the same keys. Fixes it found: `# V 21` is
  one unit value; hyphenated house numbers such as `112-10` no longer raise
  `multiple_addresses`.

### Milestone 2: parser

- `clean.rs`, `tokenize.rs`, `classify.rs`, `tagger/`, `assemble.rs`: the rule
  parser, with component spans into the original input (decision 13).
- `ParsedAddress` fields for highways, a second unit, general delivery,
  military box lines and APO/FPO/DPO (decision 15).
- `parse_fields` tags each field on its own (decision 14); `parse_many` runs
  in parallel with the `rayon` feature.
- Golden corpus format and runner (`tests/corpus/README.md`), first cases from
  auction-tracker's tests, Pub 28's examples and regressions.

### Milestone 1: data and types

- `data/*.toml` from USPS Pub 28 (Appendix B, C1, C2 and chapter 2), verified
  against the live pages; sources in `docs/usps-pub28-notes.md`.
- `build.rs` compiles the tables into static sorted arrays and validates them.
- `Lexicons` lookup interface; `StreetSuffix`, `UnitType`, `StateCode`,
  `RouteKind`; `Warning` variants with stable `code()` strings.
- `docs/prior-art.md`: lessons from the author's Florida address code.

### Milestone 0

- Workspace skeleton, CI, licenses, README, design document.
