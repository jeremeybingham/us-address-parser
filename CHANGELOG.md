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
