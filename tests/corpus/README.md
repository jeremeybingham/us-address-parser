# Golden corpus

Labeled addresses with their expected parse, one JSON record per line in
`*.jsonl`. The runner is the `doorplate::corpus` module (feature `corpus`),
run by `crates/doorplate/tests/corpus.rs` and by `doorplate corpus test`:

```sh
doorplate corpus test [--verbose] [--update-baseline]       # same runner, from the CLI
cargo test -p doorplate --test corpus -- --nocapture   # accuracy report
CORPUS_VERBOSE=1 cargo test -p doorplate --test corpus -- --nocapture   # plus field diffs
UPDATE_CORPUS_BASELINE=1 cargo test -p doorplate --test corpus   # record passing cases
```

The corpus may be ahead of the parser: a failing case does not fail the
test. `passing.txt` lists the cases that pass; the test fails if one of them
stops passing (a regression), or if the corpus is malformed. When a change
makes more cases pass, rerun with `UPDATE_CORPUS_BASELINE=1` and commit the
updated `passing.txt` with it.

Every parsing bug fixed gets a case here (`CLAUDE.md`).

## Record format

```json
{"id": "cty-street-005", "input": "4158 INVERRARY DR #309",
 "expect": {"street_number": "4158", "street_name": "INVERRARY", "street_suffix": "DR", "unit_number": "309"},
 "warnings": ["missing_city", "missing_state", "missing_zip"],
 "note": "# designator: unit_type stays empty"}
```

- `id`: unique across all files. Prefixed with the source: `cty-`
  (shapes seen in Florida county address listings, `county-data.jsonl`),
  `cty-golden-` (the same, from raw listing lines, `county-data-golden.jsonl`), `p28-`
  (Pub 28 examples), `rg-` (regressions), `key-`, `reso-` (emitter cases),
  `fl-` (Florida pack, `fl-pack.jsonl`), `pl-` (national place list outside
  Florida, `places-us.jsonl`), `ds-` (`default_state`, `default-state.jsonl`),
  `tg-` (`tag()` and `address_type()`, `tag.jsonl`),
  `co-` (trailing country, `country.jsonl`),
  `is-` (intersections, `intersections.jsonl`),
  `nat-` (shapes found in a national address database, `national-data.jsonl`);
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
    standard prefix from `crates/doorplate/data/special.toml` `[[highway]]`, plus `"state": "KY"`
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

Most cases in `county-data.jsonl`, `county-data-golden.jsonl`,
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

## Files

Ids are never a public contract; a case can be renamed or merged.

### `county-data.jsonl`

Ids `cty-street-`, `cty-full-`, `cty-placeholder-` and `cty-highway-`.
Address shapes seen in Florida county property listings, hand-labeled in this
library's schema: house number letters, units with and without designators,
highways, placeholders such as `UNKNOWN` and `NO SITUS`. Cases that need a
place list are in `fl-pack.jsonl` instead.

### `county-data-golden.jsonl`

Ids `cty-golden-001` and up. One case per distinct shape in raw address lines
from eight Florida county listings. Only the inputs were taken; the expected
values were labeled by hand, not copied from any parser's output.

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

Cases for parsing bugs found over time, from Florida address data and from
other tests (`CLAUDE.md`: every parsing bug fixed gets a case). Ids start with
`rg-`; each note says what went wrong.

### `keys.jsonl`, `reso.jsonl`

Cases that pin the emitters: exact `k1` key and slug strings (`key-`) and RESO
field values (`reso-`), written from `docs/key-format.md` and
`docs/reso-mapping.md`.

### `fl-pack.jsonl`

Ids `fl-001` to `fl-033`. The Florida pack and the national place list on
Florida input: a city on one line with no commas, a city run into the street,
cut-off cities completed, the place from the ZIP, `zip_city_mismatch`, the
observed ZIP places, `CR` as CIRCLE, `SR` as STATE ROAD, and communities and
spellings added to the pack (`docs/packs/fl.md`). `fl-013` and `fl-019` are
Georgia and Colorado inputs that show a state with no curated pack. They need
the default features (`state-fl`, `places-us`).

### `places-us.jsonl`

Ids `pl-001` to `pl-008`. The same place behaviors in states with no curated
pack (`ST LOUIS`, `NORTH LITTLE ROCK`, `LA GRANGE PARK`, `ROARING BROOK TWP`, a
cut-off city, the place from the ZIP), and `zip_city_mismatch` not raised
there.

### `default-state.jsonl`

Ids `ds-001` to `ds-007`. Cases with a `default_state` field: a comma-less
city found in the hinted state, the state's pack readings, a city not in the
hinted state, and the input's own state or a known ZIP winning over the hint.

### `country.jsonl`

Ids `co-001` to `co-015`: a trailing `USA`, `US`, `UNITED STATES` or
`UNITED STATES OF AMERICA` on one line, on its own line, in line 2, in the
city field and in the state field, each with `country_ignored`; and `US` as a
highway prefix, which is not a country.

### `national-data.jsonl`

Ids `nat-001` to `nat-023`: parser fixes found by comparing the parser with a
national address database. The addresses are invented (the database is not
redistributable and is not in the repository): words that merely begin with a
suffix spelling and end in a place name (`CREEKWOOD`, `PICO RIVERA`,
`AVENIDA DE LA LUNA`, `STRAND`), a word after `#`, values after a designator
(`APT I4`, `# 2WD`), a letter range (`APT A-D`), `U S ROUTE`, a postdirectional
`NE` before a city that starts with NE, a state code that is also a suffix
spelling (`CT`, `LA`, `WY`) settled by its ZIP, and `WASHINGTON DC`.

### `intersections.jsonl`

Ids `is-001` to `is-013`: two streets joined by `&`, `AND`, `@` or `AT`, with
the first street as components, the second in `unparsed`,
`intersection_detected`, and `none` for key and slug; plus the look-alikes that
are one street (`LAND AND SEA RD`, `BEACH AT SEA DR`, `STATE AND MAIN ST`,
`AT&T WAY`), which must not warn.

### `tag.jsonl`

Ids `tg-001` to `tg-008`. Cases with `address_type`, `tag` and `tag_repeated`:
one per address shape (street, PO Box, rural route, military, general delivery,
dual, unknown) and a repeated label (two units).
