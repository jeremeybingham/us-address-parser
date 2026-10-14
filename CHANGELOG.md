# Changelog

## Unreleased

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
