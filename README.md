# doorplate

A Rust-core US address parser with a Python package. It parses US address
strings (or pre-split fields) into components, emits **USPS Publication 28**
style and **RESO Data Dictionary** output, and produces database-ready
identifiers (a canonical match key and a URL-safe slug).

> **Status:** the Rust library, the `doorplate` CLI and the Python package
> (`doorplate`) work: parser, key, USPS, RESO and slug emitters, a
> 428-case golden corpus, a national place list plus a curated Florida pack,
> and abi3 wheels for Linux, macOS and Windows built on every pull request. On
> a blind benchmark of 30,622 one-line addresses it gets 96.4% of whole
> addresses right (coarse) against usaddress's 88.2%
> ([`docs/usaddress-comparison.md`](docs/usaddress-comparison.md)). **Nothing is
> published yet**: next is release hardening (packaging, the Pub 28
> recheck). A custom layer is deferred past the first release. `confidence` is reserved and always `None`
> in the first release (a calibrated score is deferred, `docs/design.md`
> decision 43).
> [`docs/design.md`](docs/design.md) section 10 has the plan in build order;
> section 13 has the decisions.

## This is not an address validator

This library **parses and formats** addresses. USPS Pub 28 formatting is not
the same as deliverability. It does not check that an address exists, that a
street is real. The place data (below) can say that a city and ZIP disagree
with its tables (`zip_city_mismatch`), which is a hint, not a check that the
address is right. Deliverability validation needs CASS-certified data, which
is out of scope.

## Goals

- Parse single-line and multi-field (line1, line2, city, state, zip) input.
- Emit USPS Pub 28 style and RESO field-name output.
- Produce a versioned canonical match key (`k1:...`) and a URL-safe slug.
- Report diagnostics (warnings with stable codes, unparsed remainder)
  instead of failing on partial parses; an error is returned only for
  unusable input such as an empty string.
- Usable from Rust and from Python (PyO3, abi3 wheels) with no C library or
  runtime data download.
- No machine-learned model: the parser is rules plus place data, so it is
  deterministic and a fix is a rule or a data line, not a retraining.

## Non-goals (v1)

Validation or deliverability, geocoding, fuzzy matching, address correction,
and international addresses. The shipped lexicons are fixed at compile time;
an add-only custom layer is designed but deferred past the first release
(`docs/design.md` decision 45).

## Coming from usaddress

The labels are usaddress's, so a pipeline built on them keeps working:

| usaddress | This library: Rust and CLI | Python |
|---|---|---|
| `usaddress.parse(s)`: `[(token, label)]` | `a.to_usaddress_tags()`; `doorplate parse --usaddress` | `parse_tokens(s)` |
| `usaddress.tag(s)`: `(dict, type)` | `a.tag()` and `a.tag().usaddress_type()`; `doorplate parse --tag` | `tag(s)` |
| `RepeatedLabelError` | not raised: the runs are joined and `tag().repeated_labels()` lists the labels | `parse(s).repeated_labels()` |
| `Street Address`, `PO Box`, `Ambiguous` | `a.address_type()` is finer (`Street`, `PoBox`, `RuralRoute`, `Military`, `GeneralDelivery`, `Dual`, `Unknown`); `usaddress_type()` gives the three by usaddress's own rule | `parse(s).address_type()`; `tag(s)` returns the three |
| Retrain to fix a misparse | no model to retrain; a custom layer of aliases and places is designed but deferred past the first release, so report the input to get the shipped tables fixed | the same |

What differs:

- Tokens the parser could not place have no label and are left out of
  `tag()`, and `unparsed` and a warning report them (usaddress labels every
  token).
- usaddress's `AddressNumberSuffix` (`214 B`) and number ranges
  (`5240;5220`) are kept in `AddressNumber` or left out; its intersection,
  landmark, `CornerOf`, country, recipient and `NotAddress` labels are not
  modeled (text goes to `unparsed`).
- A highway's route is `StreetName` and its prefix and state are
  `StreetNamePreType`, as in usaddress; a rural, highway contract or
  military line uses the `USPSBoxGroup*` and `USPSBox*` labels.
- Not in usaddress: `Parser::with_options(ParseOptions { default_state })`
  and `parse(s, default_state="FL")` (a state hint for input with no state),
  the Pub 28 and RESO output, the match key and slug, byte spans, `place` and
  warnings.

On the joined dicts of usaddress's own test files the two agree on 99.3% of
8,103 addresses (`docs/usaddress-comparison.md`). The feature-by-feature
comparison is in [`docs/usaddress-comparison.md`](docs/usaddress-comparison.md).

## Place data (national list and state packs)

A state pack gives the parser a state's place names, the misspellings seen
in real data, and the place each ZIP belongs to. The parser uses it to find
where the city starts on a line with no commas (`123 MAIN ST WEST PALM BEACH
FL`), to split a city run into the street (`WILSON STREETDADE CITY`), and to
fill `ParsedAddress::place`: the place the city stands for, a cut-off city
completed from the ZIP (`FERNANDINA B`), or the ZIP's place when there is no
city. `city` always keeps what the input said. Florida is the first pack
(cargo feature `state-fl`, on by default); see
[`docs/packs/fl.md`](docs/packs/fl.md) for its sources and licenses.

Under the packs sits a national list (`data/places/`, cargo feature
`places-us`, on by default): 50,375 places and 41,154 ZIPs for the 50 states,
DC and Puerto Rico, built from the Census Gazetteer and GeoNames postal codes.
The parser uses it in every state, with a curated pack (Florida) on top; the
city-versus-ZIP warning (`zip_city_mismatch`) is raised only where a pack
exists. Sources, licenses and how it is built:
[`docs/places.md`](docs/places.md). The ZIP-to-place data is GeoNames,
(c) GeoNames, CC BY 4.0 (<https://www.geonames.org/>), and is not a
statement that an address exists or can receive mail.

## Rust usage

```rust
use doorplate::{KeyOptions, Parser};

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

For data with no state (a county's listings), `Parser::with_options(ParseOptions
{ default_state: Some("FL".into()) })?` looks cities up in that state when the
input names no state and has no known ZIP. It is a hint, never a fill: `state`
stays empty, and the warning `default_state_applied` says when it decided a
place or a reading (`281 EVELYN DR DOTHAN` finds DOTHAN with `AL`). The CLI has
`--default-state ST`.

## Python usage

```python
import doorplate as uap

a = uap.parse("123 north main street apt 4, Miami FL 33101")
a.street_name, a.unit_number        # ("MAIN", "4")
a.to_usps()["line1"]                # "123 N MAIN ST APT 4"
a.key(), a.slug()                   # "k1:S|123|N|MAIN|ST||FL|33101", "123-n-main-st-apt-4-miami-fl-33101"
uap.parse_many(addresses)           # releases the GIL, parses in parallel: 6x a loop of parse
uap.tag("PO Box 12, Tampa FL 33601")  # ({...usaddress labels...}, "PO Box")
```

Typed (`py.typed`), no dependencies, Python 3.10 and later. The API, the pandas
and Polars recipes and the benchmark are in
[`crates/doorplate-py/README.md`](crates/doorplate-py/README.md). Build from source with
`maturin develop` in `crates/doorplate-py`.

## CLI

```sh
cargo run -p doorplate-cli -- parse "123 N Main St, Miami FL 33101" --pretty
cargo run -p doorplate-cli -- parse-file in.csv --column address -o out.csv
cargo run -p doorplate-cli -- parse "281 EVELYN DR DOTHAN" --default-state AL
cargo run -p doorplate-cli -- parse "123 N Main St, Miami FL 33101" --tag      # usaddress-style joined labels
cargo run -p doorplate-cli -- corpus test
```

`doorplate --help` lists every option (`addr` is a short alias for the same binary).

## Layout

```
crates/doorplate/   pure Rust library (no Python dependency)
crates/doorplate-cli/    `doorplate` command-line tool (alias `addr`)
crates/doorplate-py/     PyO3 bindings and the `doorplate` package (pytest, benchmarks)
data/               lexicon tables (Pub 28 suffixes, units, states, ...) and
                    national place list (data/places/), state
                    packs (data/packs/), compiled into static tables by
                    doorplate's build.rs
examples/           notebook, Polars and pandas pipelines, invented sample data
tests/corpus/       labeled golden corpus (JSONL)
tests/external/     usaddress's labeled test files (MIT)
tools/              corpus, comparison, place-list and pack scripts
docs/               design and format specs
.github/workflows/  ci.yml (Rust and Python tests) and wheels.yml (abi3 wheels)
```

Names (nothing is published yet; the name is reserved on the registries
before publishing, design.md decision 46): the Python package and the Rust
core crate are `doorplate`, the CLI crate is `doorplate-cli`, and the CLI
binary is `doorplate`, with `addr` as an alias.

## Documentation

- [`docs/design.md`](docs/design.md): goals, decisions, milestones, open items
- [`docs/usps-pub28-notes.md`](docs/usps-pub28-notes.md): sources for every table and rule
- [`docs/key-format.md`](docs/key-format.md): the `k1` key and the slug
- [`docs/reso-mapping.md`](docs/reso-mapping.md): RESO field mapping
- [`docs/usaddress-comparison.md`](docs/usaddress-comparison.md): accuracy against usaddress
- [`docs/packs/fl.md`](docs/packs/fl.md): the Florida pack, its sources and licenses
- [`docs/places.md`](docs/places.md): the national place list, its sources and licenses
- [`crates/doorplate-py/README.md`](crates/doorplate-py/README.md): the Python package
- [`examples/`](examples/README.md): a notebook tour and dataframe pipelines
- [`CHANGELOG.md`](CHANGELOG.md): what changed, newest first
- [`tests/corpus/README.md`](tests/corpus/README.md): the golden corpus format

## Development

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo bench -p doorplate --bench parse

# Python (in a virtualenv with maturin and pytest)
cd crates/doorplate-py && maturin develop --release && cd ../..
pytest crates/doorplate-py/tests
```

Plain `cargo test` and `clippy` need no Python: the `doorplate-py` crate has no
Rust test target, and maturin supplies the extension-module build settings.

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at
your option. The ZIP tables in `data/packs/` and `data/places/` are GeoNames
data under CC BY 4.0;
see [`NOTICE`](NOTICE).
