# us-address-parser

A Rust-core US address parser, with a Python wrapper planned. It parses US
address strings (or pre-split fields) into components, emits **USPS
Publication 28** style and **RESO Data Dictionary** output, and produces
database-ready identifiers (a stable canonical match key and a URL-safe slug).

> **Status:** the Rust library and the `addr` CLI work: parser, key, USPS,
> RESO and slug emitters, a 258-case golden corpus and a comparison with
> usaddress. Next is the place list (Milestone 5), then the Python wrapper
> (Milestone 6). Nothing is published yet. See [`docs/design.md`](docs/design.md) for the plan and open items.

## This is not an address validator

This library **parses and formats** addresses. USPS Pub 28 formatting is not
the same as deliverability. It does not check that an address exists, that a
ZIP matches a city, or that a street is real. Deliverability validation needs
CASS-certified data, which is out of scope.

## Goals

- Parse single-line and multi-field (line1, line2, city, state, zip) input.
- Emit USPS Pub 28 style and RESO field-name output.
- Produce a versioned canonical match key (`k1:...`) and a URL-safe slug.
- Report diagnostics (warnings with stable codes, unparsed remainder)
  instead of failing on partial parses; an error is returned only for
  unusable input such as an empty string.
- Usable from Rust and from Python (PyO3) with no C library or runtime data
  download.

## Non-goals (v1)

Validation or deliverability, geocoding, fuzzy matching, address correction,
international addresses, ZIP-to-city/county lookups, and runtime-configurable
lexicons.

## Rust usage

```rust
use addr_core::{KeyOptions, Parser};

let parser = Parser::new();
let a = parser.parse("123 north main street apt 4, Miami FL 33101")?;
assert_eq!(a.street_name.as_deref(), Some("MAIN"));
assert_eq!(a.unit_number.as_deref(), Some("4"));

a.to_usps().line1;                // Some("123 N MAIN ST APT 4")
a.to_reso();                      // RESO Data Dictionary 2.0 fields
a.key(KeyOptions::property());    // Some("k1:S|123|N|MAIN|ST||FL|33101")
a.slug();                         // Some("123-n-main-st-apt-4-miami-fl-33101"), display only
a.diagnostics.warnings;           // e.g. MissingZip, UnparsedRemainder
```

`Parser::parse_fields` takes pre-split fields (`Fields`), and
`Parser::parse_many` parses a batch (in parallel with the `rayon` feature).

## CLI

```sh
cargo run -p addr-cli -- parse "123 N Main St, Miami FL 33101" --pretty
cargo run -p addr-cli -- parse-file in.csv --column address -o out.csv
cargo run -p addr-cli -- corpus test
```

`addr --help` lists every option.

## Layout

```
crates/addr-core/   pure Rust library (no Python dependency)
crates/addr-cli/    `addr` command-line tool
crates/addr-py/     PyO3 bindings (planned, Milestone 6)
data/               lexicon tables (Pub 28 suffixes, units, states, ...),
                    compiled into static tables by addr-core's build.rs
tests/corpus/       labeled golden corpus (JSONL)
tests/external/     usaddress's labeled test files (MIT)
tools/              corpus and comparison scripts
docs/               design and format specs
```

## Documentation

- [`docs/design.md`](docs/design.md): goals, decisions, milestones, open items
- [`docs/usps-pub28-notes.md`](docs/usps-pub28-notes.md): sources for every table and rule
- [`docs/key-format.md`](docs/key-format.md): the `k1` key and the slug
- [`docs/reso-mapping.md`](docs/reso-mapping.md): RESO field mapping
- [`docs/usaddress-comparison.md`](docs/usaddress-comparison.md): accuracy against usaddress

## Development

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo bench -p addr-core --bench parse
```

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at
your option.
