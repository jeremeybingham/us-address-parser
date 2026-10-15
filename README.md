# us-address-parser

A Rust-core US address parser with a Python wrapper. It parses US address
strings (or pre-split fields) into components, emits **USPS Publication 28**
style and **RESO Data Dictionary** output, and produces database-ready
identifiers (a stable canonical match key and a URL-safe slug).

> **Status: Milestones 0 to 2 done (parser).** `Parser::parse`, `parse_fields` and
> `parse_many` parse single-line and multi-field input into components, with
> spans into the original text and diagnostics. The rules and their sources
> are in [`docs/usps-pub28-notes.md`](docs/usps-pub28-notes.md). `ParsedAddress::key` gives the
> canonical match key ([`docs/key-format.md`](docs/key-format.md)). `ParsedAddress::to_usps`
> gives Pub 28 style lines (layout rules checked against the live Pub 28
> text; see the notes). `ParsedAddress::to_reso` gives RESO Data Dictionary 2.0 address fields
> ([`docs/reso-mapping.md`](docs/reso-mapping.md)). `ParsedAddress::slug` gives a display-only URL slug. See
> [`docs/design.md`](docs/design.md) for the milestone plan.

## This is not an address validator

This library **parses and formats** addresses. USPS Pub 28 formatting is not
the same as deliverability. It does not check that an address exists, that a
ZIP matches a city, or that a street is real. Deliverability validation needs
CASS-certified data, which is out of scope.

## Goals

- Parse single-line and multi-field (line1, line2, city, state, zip) input.
- Emit USPS Pub 28 style and RESO field-name output.
- Produce a versioned canonical match key (`k1:...`) and a URL-safe slug.
- Report diagnostics (warnings, unparsed remainder, confidence) instead of
  failing on partial parses; an error is returned only for unusable input
  such as an empty string.
- Usable from Rust and from Python (PyO3) with no C library or runtime data
  download.

## Non-goals (v1)

Validation or deliverability, geocoding, fuzzy matching, address correction,
international addresses, ZIP-to-city/county lookups, and runtime-configurable
lexicons.

## Layout

```
crates/addr-core/   pure Rust library (no Python dependency)
crates/addr-cli/    `addr` command-line tool
crates/addr-py/     PyO3 bindings, built with maturin (placeholder)
data/               lexicon tables (Pub 28 suffixes, units, states, ...),
                    compiled into static tables by addr-core's build.rs
tests/corpus/       labeled golden corpus (JSONL)
docs/               design and format specs
```

## Rust usage (target API)

```rust
use addr_core::Parser;

let parser = Parser::new();
let parsed = parser.parse("123 north main street apt 4, Miami FL 33101")?;
assert_eq!(parsed.street_name.as_deref(), Some("MAIN"));
assert_eq!(parsed.unit_number.as_deref(), Some("4"));

// Lexicon lookups:
let lex = addr_core::Lexicons::builtin();
assert_eq!(lex.suffix("Avenue").unwrap().value.abbr(), "AVE");
assert_eq!(lex.state("Fla.").unwrap().value.code(), "FL");

// parsed.to_usps(); parsed.to_reso(); parsed.key(...);
// parsed.slug() -> "123-n-main-st-apt-4-miami-fl-33101" (display only; use key() for joins)
```

## Development

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-features
cargo run -p addr-cli -- parse "123 N Main St, Miami FL 33101"
```

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at
your option.
