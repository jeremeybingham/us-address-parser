# Golden corpus

Labeled addresses with their expected parse, one JSON record per line in
`*.jsonl`. The runner is the `addr_core::corpus` module (feature `corpus`),
run by `crates/addr-core/tests/corpus.rs` and by `addr corpus test`:

```sh
addr corpus test [--verbose] [--update-baseline]            # same runner, from the CLI
cargo test -p addr-core --test corpus -- --nocapture   # accuracy report
CORPUS_VERBOSE=1 cargo test -p addr-core --test corpus -- --nocapture   # plus field diffs
UPDATE_CORPUS_BASELINE=1 cargo test -p addr-core --test corpus   # record passing cases
```

The corpus may be ahead of the parser: a failing case does not fail the
test. `passing.txt` lists the cases that pass; the test fails if one of them
stops passing (a regression), or if the corpus is malformed. When a change
makes more cases pass, rerun with `UPDATE_CORPUS_BASELINE=1` and commit the
updated `passing.txt` with it.

Every parsing bug fixed gets a case here (`CLAUDE.md`).

## Record format

```json
{"id": "at-street-005", "input": "4158 INVERRARY DR #309",
 "expect": {"street_number": "4158", "street_name": "INVERRARY", "street_suffix": "DR", "unit_number": "309"},
 "warnings": ["missing_city", "missing_state", "missing_zip"],
 "note": "# designator: unit_type stays empty"}
```

- `id`: unique across all files. Prefixed with the source: `at-`
  (auction-tracker tests), `atg-` (auction-tracker golden CSVs), `p28-`
  (Pub 28 examples), `rg-` (regressions), `key-`, `reso-` (emitter cases),
  `fl-` (Florida pack, `fl-pack.jsonl`);
  `us-roundtrip-001` in `regressions.jsonl` came from the USPS round trip.
- Exactly one input form:
  - `input`: a single-line string, passed to `Parser::parse`.
  - `lines`: an array of address lines as the source gave them (for example
    a street line and a `CITY, ST ZIP` line). Two lines are passed to
    `Parser::parse_fields` as `line1` and `line2`, which reads them like one
    line when city, state and ZIP are empty; one line goes to `parse`, and
    more are joined with `", "`.
  - `fields`: `{line1, line2, city, state, zip}`, passed to
    `Parser::parse_fields`.
- `expect`: expected `ParsedAddress` components, named as the struct fields
  are, plus `unparsed` (`Diagnostics::unparsed`). **An absent key means that
  component must be empty**, so a parser that fills in extra parts fails.
  Structured values:
  - `rural_route`: `{"kind": "RR" | "HC", "route": "2", "box": "152"}`
  - `highway`: `{"prefix": "COUNTY ROAD", "route": "23"}`, with the
    standard prefix from `data/special.toml` `[[highway]]`, plus `"state": "KY"`
    when the name holds a state (`KY STATE HIGHWAY 1`, Appendix F)
  - `military`: `{"kind": "PSC", "number": "1650", "box": "10"}`
  - `military_post_office`: `"APO"`, `"FPO"` or `"DPO"`
  - `general_delivery`: `true` (absent means false)
  - `building`: the building name as tokens joined (`"EAST TOWER"`)
  - `attention`: the attention or care-of line as tokens joined
    (`"ATTN LAW DEPT"`, `"C/O YMF"`)
  - a dual address (street and PO Box) fills both the street fields and
    `po_box`, with the `dual_address` warning
  - `second_unit_type`, `second_unit_number`: a second unit after the
    first, in input order
- `warnings`: `Warning::code()` strings. Compared as a set, exactly: missing
  or extra warnings both fail.
- `note`: optional, why the case is there or why it is labeled this way.

Emitter expectations are optional record keys, checked only when present
(unlike `expect`, absent means not checked):

- `key`: the property-level key string (`KeyOptions::property()`), or
  `"none"` when the address has no key.
- `key_unit`: the same for `KeyOptions::unit()`.

- `slug`: the expected `slug()` string, or `"none"`.

- `usps`: `{"line1": ..., "line2": ..., "last_line": ...}` from
  `to_usps()`; an omitted part must be empty.

- `address_type`: the expected `ParsedAddress::address_type().code()`:
  `street`, `po_box`, `rural_route`, `military`, `general_delivery`, `dual`
  or `unknown`. Absent means not checked.

- `tag`: the expected `ParsedAddress::tag()`, an object from usaddress label
  to joined value (`{"AddressNumber": "123", "StreetName": "Main"}`); a label
  not listed must be absent. `tag_repeated` lists the labels `tag()` joined
  from separate runs (two units give `OccupancyType` and
  `OccupancyIdentifier`). Absent `tag` means not checked.

- `default_state`: a state (code or name) passed as
  `ParseOptions::default_state` for this case, so the hint cases (`ds-*`) can
  run in the one corpus. Absent means no hint.

- `place`: `ParsedAddress::place` as `"NAME (source)"`, with the pack's
  standard name and the `PlaceSource::code()` (`"WEST PALM BEACH (name)"`,
  `"FERNANDINA BEACH (completed)"`, `"PENSACOLA (zip)"`), or `"none"`.
  These cases need the state packs, which the tests always build with
  (default features).

- `reso`: an object of Data Dictionary 2.0 field names to the expected
  `to_reso()` values (`{"StreetName": "MAIN", "City": "MIAMI"}`). A field not
  listed must be empty. Names are checked against the Data Dictionary address
  fields; `StreetNumberNumeric` is written as its decimal text. Write expected
  values from `docs/reso-mapping.md`, not by copying the emitter's output.

Most cases in `auction-tracker.jsonl`, `auction-tracker-golden.jsonl`,
`pub28.jsonl` and `regressions.jsonl` carry `key`, `key_unit`, `slug` and
`usps` expectations, and the ones with a state and ZIP also `reso`. They were
generated from each case's hand-labeled `expect` by
`tools/corpus/emit_expect.py`, a second implementation of the rules in
`docs/key-format.md`, `docs/reso-mapping.md` and Pub 28 that never reads the
emitters, so a disagreement shows up as a failing case. Cases are left out
when the label itself is a judgment call: `unparsed` text, or the warnings
`possibly_truncated`, `ambiguous_street_number`, `multiple_addresses`,
`invalid_zip` and `ambiguous_token`. Placeholders get `none` for key and slug
and an empty `usps`. When you add a case, run the script on the file (it adds
only the fields that are missing) and review the diff. `--check` compares
every stored expectation with what the script computes and exits 1 on a
difference; CI runs it, so a rule change in the emitters or the specs has to
reach the script too.

The runner rejects unknown keys, duplicate ids, unknown warning codes, and
suffix, unit, state, directional, highway-prefix or military values that are
not standard forms from the lexicon tables.

## Labeling conventions

- Values are written as the parser should store them: uppercase, periods
  dropped, suffixes, directionals, unit designators and states as their
  standard abbreviations (`CIRCLE` -> `CIR`, `EAST` -> `E`, `UN` -> `UNIT`).
  Names keep their own words (`6th` -> `6TH`, `St. Petersburg` ->
  `ST PETERSBURG`; expanding `ST` to `SAINT` in a place name is a state-pack
  job).
- A unit with no designator (`DR 706`) or with `#` has an empty `unit_type`
  and only a `unit_number`. Pub 28 C2 uses `#` when the designator is
  unknown, so the USPS emitter writes both as `# 706`.
- The parser does not guess. A token it cannot place confidently goes to
  `unparsed` (`409742ND SQ`, `5210/5212`).
- Warnings follow these rules, so labels stay consistent:
  - `unparsed_remainder` exactly when `unparsed` is set.
  - `missing_city`, `missing_state`, `missing_zip` whenever that component is
    empty, including for street-only lines and placeholders. An APO, FPO or
    DPO in the city position counts as the city.
  - `missing_street_number` when there is a street name or highway but no
    number; `zero_street_number` replaces it for house number 0. Neither is
    raised when nothing street-like was found, or for PO Box, rural route,
    general delivery and military lines.
- A highway address keeps the prefix and route in `highway` and leaves
  `street_name` empty. Words after the route are labeled as on any street:
  `HWY 11 BYPASS` has suffix BYP, `US 41 SW` has postdirectional SW.
  - `invalid_zip` replaces `missing_zip` when there is a ZIP-like token that
    is not a valid ZIP (a cut-off `3`); the token goes to `unparsed`.
  - A placeholder line is consumed whole: `placeholder`, nothing in
    `unparsed`.

The tagger follows these conventions; its rules and their sources are in
`docs/usps-pub28-notes.md` ("Parser rules").

## Sources

### `auction-tracker.jsonl`

Hand-labeled from the author's auction-tracker repository,
`packages/radar/tests/test_addresses.py` (reviewed 2026-10-14; see
`docs/prior-art.md`). Inputs are Florida county auction listings. The
expected values restate that file's assertions in this library's schema:
auction-tracker keeps the predirectional, name and suffix in one
`street_name` string and has no unit type, so those are split here by hand.
Where that file's labels depend on its Florida place list, the case is left
out (see below).

### `auction-tracker-golden.jsonl`

Ids start with `atg-`. Hand-labeled from the raw `address_line1` /
`address_line2` columns of auction-tracker's
`packages/radar/tests/golden/v2/*.csv` (Brevard, Charlotte, Citrus, Escambia,
Hernando, Lee, Miami-Dade, Orange; reviewed 2026-10-15). Only the input
shapes were taken, one case per distinct shape; the CSVs' parsed columns
were not used as expected values. Ids `atg-006` and `atg-028` are unused, and `atg-029` was added later
(cases dropped after review: an ambiguous street-plus-ZIP round trip, and
empty input, which is an error by design decision 6 rather than a parse).

Not taken from auction-tracker:

- `tests/golden/v2/*.csv`. These are auction-tracker's own parser output,
  not hand-checked labels, so using them as expected values would copy its
  mistakes. They can be used as unlabeled inputs, or labeled after review.
- Cases that need a place list, deferred to the Florida state pack
  (`docs/design.md` section 11): a city run into the street
  (`13551 WILSON STREETDADE CITY`), a cut-off city completed from the county's
  places (`26322 BRAHMA DRIVEWESLEY CHA`, `7709 SEASHORE DRIVEPORT RICH`), and
  a city taken from the ZIP (`TAL`, `SAINT LUCIE COUNTY`).

The highway routes `8932 HWY 2301` and `3911 HIGHWAY 97` were held back
until `ParsedAddress` had a highway field; they are `at-highway-001` and
`at-highway-002`.

### `pub28.jsonl`

Built from the examples in USPS Publication 28 (`docs/usps-pub28-notes.md`
has the page for each section): the highway table in section 237 and the
"Examples in Use" column of Appendix F, the PO Box (281, 283), rural route
(241, 244), highway contract route (251, 253) and general delivery (261)
formats, and the military and Department of State examples in sections
225.1, 238.1 and 239. Inputs use the spellings Pub 28 says to change, and
the expected values are the standard forms it gives. House numbers are only
present where Pub 28's example has one.

### `regressions.jsonl`

Cases for parsing bugs found while building the tagger (`CLAUDE.md`: every
parsing bug fixed gets a case), and the multi-field shapes from
`docs/prior-art.md`. Ids start with `rg-`; each note says what went wrong.
