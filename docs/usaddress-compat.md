# usaddress compatibility

[usaddress](https://github.com/datamade/usaddress) is a Python library that
labels each token of an address with a statistical model. `doorplate` does
something similar and something different: it parses with rules and a place list,
and also standardizes (Pub 28, RESO), builds a key and a slug, and reports
diagnostics. This page is for people who already use usaddress, or who want to
compare the two. Accuracy and speed numbers are in
[`benchmarks.md`](benchmarks.md).

## Output in usaddress's shape

The labels are usaddress's, so a pipeline built on them keeps working:

| usaddress | doorplate: Rust and CLI | Python |
|---|---|---|
| `usaddress.parse(s)`: `[(token, label)]` | `a.to_usaddress_tags()`; `doorplate parse --usaddress` | `parse_tokens(s)` |
| `usaddress.tag(s)`: `(dict, type)` | `a.tag()` and `a.tag().usaddress_type()`; `doorplate parse --tag` | `tag(s)` |
| `RepeatedLabelError` | not raised: the runs are joined and `tag().repeated_labels()` lists the labels | `parse(s).repeated_labels()` |
| `Street Address`, `PO Box`, `Ambiguous` | `a.address_type()` is finer (`Street`, `PoBox`, `RuralRoute`, `Military`, `GeneralDelivery`, `Dual`, `Unknown`); `usaddress_type()` gives the three by usaddress's own rule | `parse(s).address_type()`; `tag(s)` returns the three |
| Retrain to fix a misparse | no model to retrain; report the input so the shipped tables or rules are fixed | the same |

What differs:

- Tokens the parser could not place have no label and are left out of `tag()`;
  `unparsed` and a warning report them (usaddress labels every token).
- usaddress's `AddressNumberSuffix` (`214 B`) and number ranges (`5240;5220`) are
  kept in `AddressNumber` or left out. Its intersection, landmark, `CornerOf`,
  country, recipient and `NotAddress` labels are not modeled: that text goes to
  `unparsed`. A trailing `USA` is ignored with `country_ignored`, and an
  intersection raises `intersection_detected`.
- A highway's route is `StreetName` and its prefix and state are
  `StreetNamePreType`, as in usaddress. A rural, highway contract or military line
  uses the `USPSBoxGroup*` and `USPSBox*` labels.
- Not in usaddress: `default_state` (a state hint for input with no state), the
  Pub 28 and RESO output, the match key and slug, byte spans, `place` and warnings.

## Label mapping

| doorplate | usaddress label |
|---|---|
| street number | `AddressNumber` |
| predirectional, postdirectional | `StreetNamePreDirectional`, `StreetNamePostDirectional` |
| street name | `StreetName` |
| suffix | `StreetNamePostType` |
| highway: state and prefix; route | `StreetNamePreType`; `StreetName` |
| unit designator, value (both units) | `OccupancyType`, `OccupancyIdentifier` |
| PO Box word, number | `USPSBoxType`, `USPSBoxID` |
| rural, highway contract or military line: kind, number, BOX, box number | `USPSBoxGroupType`, `USPSBoxGroupID`, `USPSBoxType`, `USPSBoxID` |
| general delivery | `USPSBoxType` |
| city, APO/FPO/DPO | `PlaceName` |
| state | `StateName` |
| ZIP, ZIP+4 | `ZipCode` |
| attention line | `Recipient` |
| building name | `BuildingName` |
| unparsed text, skipped copies | no label |

A `#` takes the label of the token after it, as usaddress's data does (`# 234` is
two `OccupancyIdentifier` tokens).

## Feature comparison

| Area | usaddress | doorplate |
|---|---|---|
| Install and call | `pip install`; `parse`, `tag` | Rust crate, CLI and a Python package (`parse`, `parse_many`, `tag`, `parse_tokens`); abi3 wheels for Linux, macOS and Windows |
| Token labels | `parse()` | `to_usaddress_tags()`; `parse_tokens()` in Python |
| Joined components and type | `tag()` returns a dict and `Street Address` / `PO Box` / `Ambiguous`; raises `RepeatedLabelError` | `tag()`, `address_type()`; a repeated label is reported, not an error |
| Input forms | One string | One string or fields (`line1`, `line2`, city, state, ZIP) |
| Default state | None | `default_state` hint, `--default-state` |
| Normalized output | None (see `usaddress-scourgify`) | USPS Pub 28, RESO, key, slug |
| Diagnostics, spans, confidence | None (CRF marginals only) | Warnings, unparsed text, byte spans, `place`; `confidence` is reserved and always `None` |
| Rare address shapes | Basic | Rural and highway routes, military, general delivery, dual, second unit |
| Customization | Retrain with parserator on your own labels | Fixed at compile time |
| Intersections, landmarks, `CornerOf`, country, recipient names | Labeled | Not modeled; text goes to `unparsed`. A trailing country is ignored with `country_ignored` and an intersection raises `intersection_detected` |
| Display casing, long form | Keeps input case | Uppercase only |
| Batch and dataframes | Loop yourself | `parse_many` (parallel; GIL released in Python); `pandas.parse_column` and `polars.parse_column` helpers (optional extras) |

## How the two are compared

`tools/compare/compare_usaddress.py` runs both parsers on the same strings and
scores them by the same rules. `doorplate` produces usaddress's labels with
`doorplate parse --usaddress`, built from the component spans without re-parsing.

```sh
cargo build --release -p doorplate-cli
python tools/compare/compare_usaddress.py gold tests/external/usaddress/*.xml --out report.md
python tools/compare/compare_usaddress.py raw data.csv --column address \
    [--line2 C --city C --state C --zip C] [--no-header] --out report.md
python tools/compare/compare_usaddress.py tag tests/external/usaddress/*.xml --out tag-report.md
```

`gold` scores labeled files, `raw` reports agreement on unlabeled data, and `tag`
compares the joined dicts of `tag()` with `usaddress.tag()` and the address types.
usaddress's labeled test files are vendored in `tests/external/usaddress/` (MIT).

### Scoring rules

- Tokens are compared one by one; tokens with no letter or digit (`-`, `&`) are
  not scored.
- **Out of scope:** addresses whose gold labels include `Recipient`,
  `LandmarkName`, `CornerOf`, `IntersectionSeparator`, `NotAddress`,
  `CountryName`, `SubaddressType`, `SubaddressIdentifier`, `StreetNamePreModifier`
  or `AddressNumberPrefix` are left out; this parser does not model them.
- **Same thing, different label:** `AddressNumberSuffix` counts as `AddressNumber`
  on both sides. In raw mode, usaddress's `SubaddressType` and `SubaddressIdentifier`
  count as `OccupancyType` and `OccupancyIdentifier`.
- **Coarse score:** the five street-part labels fold into `StreetName`.
  `us50_test_tagged.xml` labels a whole street as `StreetName` (`Mendenhall Mall
  Road,`), so only the coarse score means anything there. The fine score is
  reported too.
- **Training overlap:** addresses listed in `tests/external/usaddress/in_training.txt`
  (made by `tools/compare/training_overlap.py`) are reported apart.

### Why usaddress's own tests are not a contest

usaddress's CI asserts the labels in its test files on every release, and its
comments say new formats go in both training and test data. So it scores about
100% on them even where an address is not in its training files. Those numbers
say how close doorplate comes to usaddress's behavior, not which is more
accurate. A blind comparison needs labels neither parser has seen
([`benchmarks.md`](benchmarks.md) uses a public geocoding benchmark).

### How closely `tag()` agrees

On the strings of usaddress's test files that use only labels doorplate models
(8,108 addresses, 6,658 of them in usaddress's training files), the joined dicts
are identical on 8,043 of 8,103 compared (99.3%), and the address type on 99.8%.
The 60 dicts that differ are boundary choices: `SAINT JOHN W ST` (the `W` in the
name or a postdirectional), a `BUSINESS` or `LH` suffix word, `TWP` in the city, a
number range (`5240;5220`) or an `AddressNumberSuffix` (`214 B`), and labels
doorplate does not model (`church street station` as `BuildingName`). The type
differences are addresses with no `AddressNumber` or `USPSBoxID` here. Rerun the
`tag` mode after a parser change; it prints every label that differs.
