# Changelog

## Unreleased

- RESO emitter (Milestone 3): `ParsedAddress::to_reso()` returns a
  `ResoAddress` with the RESO Data Dictionary 2.0 Property address fields
  (`StreetNumber`, `StreetNumberNumeric`, `StreetDirPrefix`, `StreetName`,
  `StreetSuffix`, `StreetDirSuffix`, `UnitNumber`, `City`, `StateOrProvince`,
  `PostalCode`, `PostalCodePlus4`, `UnparsedAddress`). `fields()` gives
  Data Dictionary names; `length_overruns()` reports values over the
  suggested maximum lengths. `docs/reso-mapping.md` has the sources, the
  folding rules for shapes with no field, and open questions for review.
  Golden corpus: optional `reso` expectation and `tests/corpus/reso.jsonl`.
- Initial workspace skeleton (Milestone 0).
- Milestone 1, data and types:
  - Populated `data/*.toml` from USPS Pub 28 (Appendix B, C1, C2 and
    chapter 2), cross-checked against open-source transcriptions; sources and
    verification status in `docs/usps-pub28-notes.md`.
  - `build.rs` compiles the tables into static sorted arrays and validates them.
    Runtime has no dependencies.
  - `Lexicons` lookup interface (suffix, directional, unit, state, special
    forms, highway prefixes, placeholder text).
  - Types: `StreetSuffix`, `UnitType` (with `range_required`), `StateCode`
    (with `StateKind`), `RouteKind`; new `Warning` variants with stable
    `code()` strings.
  - `docs/prior-art.md`: lessons from the author's Florida address code
    (auction-tracker, dbpr_fl).
- Golden corpus (brought forward from Milestone 4):
  - JSONL record format and labeling conventions (`tests/corpus/README.md`).
  - Runner `crates/addr-core/tests/corpus.rs`: per-field and whole-address
    accuracy report; fails on malformed records and on regressions against
    `tests/corpus/passing.txt`.
  - 67 cases hand-labeled from auction-tracker's address tests.
  - `Warning::ALL`, used to check expected warning codes.
- `docs/design.md`: decisions 13 to 15 (spans into the original input,
  per-field parsing of `Fields`, new fields for highway, general delivery,
  military and second-unit addresses).
- `ParsedAddress` fields from decision 15: `highway` (prefix and route),
  `second_unit_type` / `second_unit_number`, `general_delivery`, `military`
  (CPR, OPC, PSC, UPR, UNIT box lines) and `military_post_office` (APO,
  FPO, DPO), with `data/special.toml` tables and `Lexicons::military`,
  `Lexicons::military_post_office` (Pub 28 sections 225.1, 238.1, 239).
- Component spans into the original input (decision 13): `Span`,
  `Component`, `SourceInfo::spans`, `SourceInfo::text`.
- Corpus: the two held-back highway cases, and `pub28.jsonl` with 49 cases
  from Pub 28's own examples.
- Milestone 2, parser (items 10 to 16):
  - `clean.rs`: whitespace, accents, Unicode dashes and quotes, uppercase,
    with a byte map back to the original input.
  - `tokenize.rs`: tokens with original spans and comma segments; `#`,
    parentheses, hyphens and periods; splits glued tokens (`101BRADDOCK`,
    `TAMPA33616`, `I10`, `APT107`, `CTAPT`).
  - `classify.rs`: token shapes and longest-match lexicon lookups.
  - `tagger/`: rule tagger for the last line, PO Box, rural and highway
    contract routes, general delivery, military lines, highways, streets
    and up to two units. Rules and sources: `docs/usps-pub28-notes.md`.
  - `assemble.rs`: tags to `ParsedAddress`, component spans, warnings.
  - `parse_fields` tags each field on its own (decision 14); with only
    address lines it reads them like one line.
  - `parse_many` runs in parallel with the `rayon` feature (now a real
    optional dependency).
  - `Warning::NotImplemented` removed.
  - Corpus: 134 cases, all passing (`regressions.jsonl` added).
- Milestone 3, canonical key (`k1`): `ParsedAddress::key(KeyOptions)` in
  `emit/key.rs`, spec in `docs/key-format.md`; corpus records can carry
  `key` and `key_unit`, with `tests/corpus/keys.jsonl` (7 cases).
- Milestone 3, USPS emitter: `ParsedAddress::to_usps()` returns a
  `UspsAddress` (`line1`, `line2`, city, state, ZIP, `last_line()`); corpus
  records can carry `usps`; a corpus test checks that parsing the USPS output
  gives the same keys. Layout rules are unverified against Pub 28 (blocked
  host); see `docs/usps-pub28-notes.md`.
- Parser fix found by that round trip: `# V 21` (letter and number after
  `#`) is one unit value (`us-roundtrip-001`).
- USPS emitter checked against the live Pub 28 text (items 1 to 8 of the
  verification list in `docs/usps-pub28-notes.md`; the release re-check of
  the lexicon tables stays open for Milestone 7):
  - The unit stays at the end of the delivery line (section 213); the
    40-character placeholder limit is gone, since Pub 28 sets none (35).
    New `to_usps_wrapped(max_line)` moves the unit to `line2`, and
    `UspsAddress::lines()` returns mailpiece order, with `line2` above
    `line1` (213.3, 356).
  - A period between digits is kept (`39.2 RD`, section 232).
  - `Highway::state`: a state named in a highway name is parsed and written
    as its code (`KENTUCKY STATE HIGHWAY 625` is `KY STATE HIGHWAY 625`,
    `KY 1207` is `KY HIGHWAY 1207`; Appendix F). The `k1` key name segment
    carries the code the same way.
  - Hyphenated house numbers such as `112-10` (Appendix D1) no longer raise
    `multiple_addresses`; ranges such as `1204-1206` still do.
  - Corpus: 17 new Pub 28 cases (159 in all), and `usps` checks on the
    military, general delivery, route and PO Box cases; highway records
    take an optional `state`.
