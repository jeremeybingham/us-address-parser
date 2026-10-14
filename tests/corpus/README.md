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
    a street line and a `CITY, ST ZIP` line). The runner joins them with
    `", "` and calls `Parser::parse` until multi-line input has its own entry
    point; the record keeps the source's line split for when it does.
  - `fields`: `{line1, line2, city, state, zip}`, passed to
    `Parser::parse_fields`.
- `expect`: expected `ParsedAddress` components, named as the struct fields
  are, plus `unparsed` (`Diagnostics::unparsed`). **An absent key means that
  component must be empty**, so a parser that fills in extra parts fails.
  `rural_route` is `{"kind": "RR" | "HC", "route": "2", "box": "152"}`.
- `warnings`: `Warning::code()` strings. Compared as a set, exactly: missing
  or extra warnings both fail.
- `note`: optional, why the case is there or why it is labeled this way.

Fields that the emitters will need (USPS lines, RESO fields, key) are added
in Milestone 3, as optional keys.

The runner rejects unknown keys, duplicate ids, unknown warning codes, and
`street_suffix`, `unit_type`, `state` or directional values that are not
standard abbreviations from the lexicon tables.

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
    empty, including for street-only lines and placeholders.
  - `missing_street_number` when there is a street name but no number;
    `zero_street_number` replaces it for house number 0. Neither is raised
    when nothing street-like was found.
  - `invalid_zip` replaces `missing_zip` when there is a ZIP-like token that
    is not a valid ZIP (a cut-off `3`); the token goes to `unparsed`.
  - A placeholder line is consumed whole: `placeholder`, nothing in
    `unparsed`.

These conventions describe the planned behavior; review them along with the
tagger (Milestone 2).

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
- Highway routes (`8932 HWY 2301`, `3911 HIGHWAY 97`). `ParsedAddress` has
  no field for a highway prefix yet; these are added with that field.
