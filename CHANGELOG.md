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
