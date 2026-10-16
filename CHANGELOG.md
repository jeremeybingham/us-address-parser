# Changelog

Nothing is released yet. Entries are newest first.

## Unreleased

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

### usaddress comparison (design decision 11; Milestone 8, item 35)

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
