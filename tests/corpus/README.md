# Golden corpus

Labeled addresses with their expected parse, one JSON record per line in
`*.jsonl`. The runner is `crates/addr-core/tests/corpus.rs`:

```sh
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

- `id`: unique across all files. Prefix with the source (`at-` for
  auction-tracker).
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

- `usps`: `{"line1": ..., "line2": ..., "last_line": ...}` from
  `to_usps()`; an omitted part must be empty.

- `reso`: an object of Data Dictionary 2.0 field names to the expected
  `to_reso()` values (`{"StreetName": "MAIN", "City": "MIAMI"}`). A field not
  listed must be empty. Names are checked against the Data Dictionary address
  fields; `StreetNumberNumeric` is written as its decimal text. Write expected
  values from `docs/reso-mapping.md`, not by copying the emitter's output.

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
