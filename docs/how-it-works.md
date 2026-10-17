# How doorplate works

What the library does, how an address moves through it, and why it behaves the
way it does. For usage see the [README](../README.md); for the exact output
formats see the reference pages listed at the end.

## What it is for

`doorplate` turns a US address, written as one line or as separate fields, into:

1. **Components** (street number, directionals, street name, suffix, units, city,
   state, ZIP and the less common shapes below), each with its position in the
   original text.
2. **Normalized output** in USPS Publication 28 form and RESO Data Dictionary 2.0
   field names.
3. **Identifiers for databases**: a stable match key and a URL-safe slug.
4. **Diagnostics**: warnings and the text it could not place, so bulk users can
   send doubtful rows to review.

It is a Rust library with no runtime dependencies, a command-line tool and a
Python package (PyO3, abi3 wheels for Python 3.10 and later). Nothing is
downloaded at run time.

**What it does not do.** It formats and parses; it does not validate. Pub 28
formatting is not the same as a deliverable address, and real validation needs
postal-service-certified data. It does not geocode, match fuzzily, correct
addresses or handle international addresses. When the place list finds a city
for an address, the parsed `city` stays what the input said and the match is
reported beside it (`place`).

## The pipeline

```
raw input
  -> clean      whitespace, accents, punctuation, uppercase; a map back to the original
  -> tokenize   tokens with original spans; lines and comma segments; glued tokens split
  -> classify   token shapes and longest-match lexicon lookups
  -> tagger     rules and place lookups
  -> assemble   ParsedAddress, component spans, warnings
  -> emitters   USPS, RESO, key, slug, usaddress tags (read-only)

Lexicons (Pub 28 tables, national places, state packs) are read by classify, the
tagger and assemble.
```

1. **clean** produces uppercase, single-spaced text and keeps a map from every
   cleaned byte to the original input, so components can report spans into the
   text the caller gave.
2. **tokenize** splits the cleaned text into tokens that keep their spans.
   Commas and semicolons separate *segments* (address lines or parts of one),
   which the tagger uses as strong boundaries. It splits tokens that were glued
   together when the pieces are unambiguous (`101BRADDOCK`, `APT.1285`,
   `178ST`).
3. **classify** applies shape tests to single tokens (numbers, ZIPs, ordinals,
   unit values) and longest-match lookups of runs of words in the lexicon.
4. **tagger** decides what each token is. The last line (ZIP, state, city) is
   read first, right to left, then the delivery line from both ends: units and
   postdirectional from the right, house number and predirectional from the left
   (Pub 28 section 231). It uses the place list to find a city on a line with no
   commas.
5. **assemble** turns runs of tokens into a `ParsedAddress`, looks up the
   standard values, and raises warnings.
6. **emitters** read the parsed address and nothing else. They never re-parse
   and never change it.

With separate fields (`parse_fields`), each field is tagged on its own and the
field is a strong signal: the city field is the city. When city, state and ZIP
are all empty, `line1` and `line2` are read like one line, so a street line plus
a `CITY, ST ZIP` line still works.

## Principles

- **Deterministic.** The same input gives the same output on every platform.
- **Never panics on input.** Parsing always returns a `ParsedAddress` with
  diagnostics. Only input with no address text at all is an error. Property tests
  and fuzz targets check this.
- **Spans are preserved.** Components keep their byte range in the original
  input (and the field, for separate fields).
- **Lexicons behind one interface.** The tagger never reads a table directly:
  everything goes through `Lexicons`, which merges the Pub 28 tables, the national
  place list and any state pack. There is no state-specific logic in the tagger.
- **Emitters are read-only.** A new output format cannot change a parse.
- **Rules plus a place list, no model.** There is no statistical model, no
  training and no model file. The place list is what separates a city from street
  words on a line with no commas (`281 EVELYN DR DOTHAN`); a statistical tagger
  has no place list and would not find the city either. Fixing a misparse means
  adding a rule or a table entry, which shows up in review.
- **No global state.** A `Parser` is cheap and `Send + Sync`.

## What comes out

`ParsedAddress` holds the street number, pre- and postdirectional, street name,
suffix, `highway` (state, prefix and route), first and second unit (type and
number), `po_box`, `rural_route` (RR or HC, route, box), `general_delivery`,
`military` and `military_post_office`, `attention`, `building`, city, state, ZIP5,
ZIP+4, `place`, `diagnostics` and `source` (the original text and the spans).
`Diagnostics` holds the warnings, the unparsed remainder, the library version and
`confidence`, which is reserved and always `None`.

Warnings have a stable snake_case `code()` meant for a database column:
`unparsed_remainder`, `ambiguous_token`, `placeholder`, `missing_street_number`,
`missing_street_name`, `zero_street_number`, `ambiguous_street_number`,
`multiple_addresses`, `dual_address`, `possibly_truncated`, `missing_unit_number`,
`non_standard_abbreviation`, `missing_city`, `missing_state`, `unknown_state`,
`missing_zip`, `invalid_zip`, `zip_city_mismatch`, `default_state_applied`,
`repeated_label`, `country_ignored` and `intersection_detected`. A warning is a
finding about the parse, not a statement that the address is wrong.

## Choices that shape the output

- **The place is reported beside the city, not in it.** `city` is what the input
  said (it has no span when it was found from the ZIP); `place` holds the
  standard place name and how it was found: exact, a listed variant, completed (a
  cut-off name), or from the ZIP. Filling in or correcting a city is a
  correction, which the library avoids, and the key leaves the city out.
- **City and ZIP disagreeing is a warning** (`zip_city_mismatch`), not an error.
  It says the pair does not match the place list. It fires only for states with a
  curated pack, because the national list gives one place per ZIP while the postal
  service accepts several city names for many ZIPs.
- **`default_state` is a lookup hint, never a fill.** When the input names no state
  and has no known ZIP, a city is looked up in the hinted state and the state's
  pack readings apply. `state` stays empty, `missing_state` still fires and the
  key keeps its `k1n` form; `default_state_applied` says when the hint decided
  something.
- **Dual addresses keep both.** A record with a street address and a PO Box fills
  the street fields and `po_box` and raises `dual_address`. The key, the slug and
  the RESO output use the street; the USPS output writes the PO Box alone, as Pub
  28 section 215 prefers for the mailpiece.
- **Shapes with no RESO field are folded in.** Highway routes, general delivery,
  military lines and second units have fields in `ParsedAddress`; the RESO emitter
  writes them into `StreetName` or `UnitNumber` (`docs/reso-mapping.md`).
- **Attention lines are a field** (`ATTN`, `C/O`) and are not written by any
  emitter.
- **A trailing country is ignored.** `USA`, `US` or `UNITED STATES` after the
  address is dropped with `country_ignored`; other countries are not recognized.
- **Intersections have no key.** `MAIN ST & 1ST AVE` raises
  `intersection_detected`: the first street is parsed, the second stays in
  `unparsed`, and `key()` and `slug()` are `None`, because the first street alone
  would match every address on it.
- **`tag()` and `address_type()`** give usaddress-shaped output. Tokens that share
  a label are joined; a repeated label is joined and reported (`repeated_label`)
  instead of raising an error (`docs/usaddress-compat.md`).
- **A penthouse after a designator is one unit** (`APT PH 5213` is `APT PH5213`).
- **A curated pack wins over the national list for its state.** National rows for
  that state only add places the pack lacks.

## The key

The canonical key is a deterministic string for deduplication and joins:
`123 N. Main Street, Apt 4` and `123 north main st #4` give the same key. It uses
the street components, state and ZIP5 and leaves out the city, which is the noisiest
field. Addresses missing a state or ZIP get the weaker `k1n:` prefix and a warning.
Unit inclusion is an option (off by default, property level).

The key format is versioned (`k1:`). Any change to what an input produces needs a
new version prefix (`k2:`) and an update to `docs/key-format.md`; `k1` is never
edited in place. This is an alpha release, so expect new key versions as the parser
improves; store the version with your keys. The slug is built from the
USPS output, is for display, and is not unique or versioned.

## Lexicon layers

Data comes in layers, each a set of files compiled in by `build.rs` into static,
sorted tables, so lookups are a key normalization plus a binary search with no
runtime data loading. `build.rs` fails the build on a malformed or contradictory
table.

1. **Base lexicons** (`crates/doorplate/data/*.toml`): Pub 28 and RESO tables
   (suffixes, units, states, directionals, special words), each with its source.
   Spellings Pub 28 does not list but real data uses are kept apart as `observed`
   and raise `non_standard_abbreviation`.
2. **National place list** (`data/places/`, cargo feature `places-us`, on by
   default): generated for every state, DC and Puerto Rico from public data
   (Census Gazetteer, GeoNames ZIPs). It finds cities and matches ZIPs everywhere;
   it has no misspellings. `docs/places.md`.
3. **Curated state packs** (`data/packs/<st>/`, feature `state-<st>`): hand-checked
   data for one state, added where real data shows a need. Florida is the first and
   is on by default. `docs/packs/fl.md`.

Layering is add-only: a later layer adds names, variants and ZIPs and never replaces
or removes an earlier entry, and a conflict fails the build. The one exception is a
pack's `overrides.toml`, which can read a spelling the base table has as another
suffix in that state (`CR` is CIRCLE in Florida) or add a highway prefix (`SR` is
STATE ROAD); each entry names its evidence. The shipped tables and packs are fixed
at compile time.

Cargo features: `rayon` (parallel `parse_many`), `corpus` (the golden-corpus runner;
pulls in serde), `places-us` and `state-fl` (both on by default). The default build
has no runtime dependencies.

## Lessons from real address data

The warnings and many rules exist because real county and licensing data is
messier than examples:

- Statuses make bulk data usable, so users can filter and count by warning code.
- Placeholder text (`NO SITUS`, `NOT ASSIGNED`, `0 UNKNOWN`) is common and must not
  be parsed as a street.
- House number 0 means no number, usually vacant land.
- Several addresses in one field (`5210/5212 BRISTO ST`) and numbers run into a
  numbered street (`409742ND SQ`) cannot be split reliably, so they are reported
  rather than guessed at.
- County sites cut lines at about 30 characters, mid-street or mid-unit
  (`possibly_truncated`).
- A unit word with nothing after it (`... DR #`) is reported (`missing_unit_number`).
- Real data uses spellings Pub 28 does not list (`TRC`, `UN`, `LT`).
- The number after a highway word is the road's, not a unit (`8932 HWY 2301`).
- A suffix word can start a street name (`2108 AVENUE G`), and the letter after it
  is the name.
- Cities run into the street line, with or without a space, are only safe to split
  with a place list.

## Repository layout

```
Cargo.toml                  workspace: doorplate, doorplate-cli, doorplate-py
crates/
  doorplate/                pure Rust, no Python dependency
    build.rs                compiles data/ into static tables
    data/                   suffixes, directionals, units, states, special (.toml);
                            places/ (national list) and packs/fl/ (Florida)
    src/                    clean, tokenize, classify, tagger/, assemble, parser, input,
                            place, lexicon, types, emit/{key,usps,reso,slug,usaddress}, corpus
    tests/                  corpus, property tests, usaddress score floor
    benches/                criterion benchmarks
  doorplate-cli/            the `doorplate` binary (alias `addr`)
  doorplate-py/             PyO3 bindings, the Python package, its tests
tests/corpus/               golden corpus (JSONL) and its baseline
tests/external/usaddress/   usaddress's labeled test files (MIT, vendored)
tools/                      corpus emitter expectations, usaddress comparison, place list, Pub 28 checks
benchmarks/                 accuracy, speed and size scripts
examples/                   notebook, Polars and pandas pipelines, sample data
fuzz/                       cargo-fuzz targets (nightly)
docs/                       this page and the references below
```

## Testing

- **Golden corpus** (`tests/corpus/*.jsonl`): expected components and warnings; most
  cases also carry the key, slug, USPS and RESO output, written from the specs by
  `tools/corpus/emit_expect.py` (an independent implementation of the rules, so a
  disagreement shows up as a failing case). The runner fails on malformed records
  and on regressions against `passing.txt`. The corpus is hand-labeled and the rules
  were tuned against it, so 100% on it is a regression check, not an accuracy
  measure.
- **USPS round trip**: parsing the USPS output of a parsed address gives the same key.
- **Property tests** (proptest): no panic, valid spans, every emitter, `parse_many`
  consistent with a loop.
- **usaddress score floor**: this parser on usaddress's labeled test files must not
  fall below a recorded score.
- **Fuzzing** (cargo-fuzz, nightly): `parse` (with a `default_state` hint) and
  `parse_fields`, each running every emitter on the result.
- **Python tests**: the wrapper, type stubs, `parse_many`, the dataframe helpers,
  the examples. **Wheels** are built and tested for Linux, macOS and Windows.
- **Pub 28 check**: `tools/pub28/verify.py` compares the tables with the live
  Publication 28 pages and runs every Appendix F example through the parser.
- **CI**: `cargo fmt --check`, `clippy -D warnings`, `cargo test --workspace
  --all-features`, the corpus report, the emitter-expectation check, a package
  build, the Python suite and the wheel build.

## Reference pages

| Page | Contents |
|---|---|
| [`key-format.md`](key-format.md) | The `k1` key and the slug: the contract |
| [`reso-mapping.md`](reso-mapping.md) | RESO Data Dictionary field mapping |
| [`usps-pub28-notes.md`](usps-pub28-notes.md) | The source of every table and parser rule |
| [`places.md`](places.md) | The national place list: sources, licenses, coverage |
| [`packs/fl.md`](packs/fl.md) | The Florida pack |
| [`usaddress-compat.md`](usaddress-compat.md) | usaddress-shaped output, label mapping, migration |
| [`benchmarks.md`](benchmarks.md) | Accuracy, speed and size |
| [`roadmap.md`](roadmap.md) | Planned work and what data would help |
