# Roadmap

What is planned, what depends on what, and what is deliberately not planned.
`doorplate` 0.1.0 is an alpha. Priorities follow the data people bring: the more real,
messy addresses it sees, the sooner the items below that need them can happen. To send
some, see [Sharing messy data](../CONTRIBUTING.md#sharing-messy-data).

## Stability while it is an alpha

- **Keys.** The match key is versioned (`k1:`) and each version is a contract
  ([`key-format.md`](key-format.md)), but expect new versions. A parser fix that changes
  what an input produces changes its key, and that needs a new version, so `k2:` and later
  are likely as the parser improves. Store the version prefix with the keys.
- **API and warning codes.** Names may change before 1.0, with a note in the changelog.
- **Accuracy and speed** are measured on a few datasets ([`benchmarks.md`](benchmarks.md))
  and are not guaranteed for yours.

## Planned

### Confidence score

`Diagnostics::confidence` is reserved and always `None`. The goal is a calibrated
probability that the whole parse is right: of the addresses scored 0.9, about 90% are
right on a labeled set the rules were not tuned on. The tagger already records the
uncertain decisions it makes (a city found by comma or by suffix, a state with no ZIP, a
bare unit, ...) as the input for a score. What is missing is **labeled data at scale**:

- messy addresses with correct, hand-checked answers, especially the rare paths
  (`unparsed_remainder`, `missing_street_number`, bare units, cities found by suffix);
- structured sources that keep line 1, line 2, city, state and ZIP in separate columns,
  which give automatic labels for those components.

A score ships only if its calibration holds on held-out data; otherwise it stays `None`.

### Customization packs

A way to add your own spellings, places, ZIPs and state-scoped readings without editing
the shipped tables: an add-only layer in the state-pack format, loaded at run time,
validated on load, deterministic, and reported in the output so stored keys can be tied to
the layer in use. Today the tables are fixed at compile time, and a misparse is fixed by
reporting it. The design is settled; it waits for real examples of what people need to add
(a local community the place list lacks, a street spelling, a state-specific suffix).

### More state packs

Florida is the first curated pack. Another state's pack starts from real data for that
state. Known candidates: `RR` as Ranch Road in Texas, `PR 462` (a Texas private road or a
Puerto Rico road), county road names such as `LEE ROAD 2004`, Queens-style hyphenated house
numbers, and Puerto Rico urbanizations.

### More shapes and rules

Known gaps that are not tied to a state:

- named buildings without a building word (`THE GARRETT`, `UPTOWN SQUARE`);
- highway modifiers before a city on one line (`NORTH HIGHWAY 71 BUSINESS LOWELL`);
- a comma after a directional (`3324 N, ANDREWS AVE`);
- a PO box on line 2 beside a street with no number;
- a unit followed by business words (`STE 903 BLU SPA`);
- `possibly_truncated` firing on some complete units.

Whether to model more of what usaddress labels (recipient and name lines, landmarks,
street corners with cross-street fields) is open; today that text goes to `unparsed`.

### More output

- A mixed-case display form (`123 N Main St`) and a long form (`123 North Main Street`).
  Neither is a Pub 28 form; they would be labeled display only.
- A batch call for pre-split fields (`parse_fields_many`) and a columnar or Arrow result
  for DataFrames: reading attributes from Python objects is most of the time in a pandas or
  Polars pipeline.
- A Polars expression namespace, if there is demand.

### The key

Candidates for `k2`: how leading zeros in route numbers are written (`RR 02` and `RR 2` have
different `k1` keys today), and anything the messy data shows.

### Data and benchmarks

- A benchmark built from a national address source (NAD or similar), rendered in several
  styles and compared per state and per shape, to find parser bugs and place-list gaps.
- A cleaner blind test: a hand-labeled sample of real messy data in the shapes people
  actually have.
- A profile of `parse`: it is slower than an earlier build, and one address takes 10 to 23
  microseconds in Rust, so there is probably a cheap win in the lexicon lookups.
- The pack infrastructure: report a pack's version in the output, and require a golden
  corpus per pack in CI.

## Not planned

- **Address validation or deliverability.** That needs postal-service-certified data.
- **Geocoding, fuzzy matching, address correction.**
- **International addresses.**
- **A machine-learned model.** The parser is rules plus place data on purpose, so a fix is a
  rule or a line of data, and the same input gives the same output.
