# Comparison with usaddress

How this parser is compared with the Python
[usaddress](https://github.com/datamade/usaddress) library (design sections 9
and 10), and the results.

## Tools

- `ParsedAddress::to_usaddress_tags()` (`crates/addr-core/src/emit/usaddress.rs`):
  every input token with the label usaddress would use. Built from the
  component spans; it never re-parses.
- `addr parse --usaddress`: those tags as JSON, one record per input line.
- `tools/compare/compare_usaddress.py`: runs both parsers on the same strings
  and scores them by the same rules.

  ```
  cargo build --release -p addr-cli
  uv run --with usaddress python tools/compare/compare_usaddress.py \
      gold tests/external/usaddress/*.xml --out report.md
  uv run --with usaddress python tools/compare/compare_usaddress.py \
      raw data.csv --column address [--line2 C --city C --state C --zip C] \
      [--no-header] --out report.md
  ```

- `crates/addr-core/tests/usaddress_gold.rs`: scores this parser alone on
  the vendored test files in `cargo test`, and fails if the score falls below
  the recorded floor. No Python needed.

Test data: `tests/external/usaddress/` (MIT, pinned commit; see its README).

## Feature comparison

What each library offers, apart from accuracy (reviewed 2026-10-16; the
plan items are in `docs/design.md`).

| Area | usaddress | This library | Plan |
|---|---|---|---|
| Install and call | `pip install`; `parse`, `tag` | Rust crate and CLI; Python wrapper not built | Milestone 6 |
| Token labels | `parse()` | `to_usaddress_tags()` | Done; `parse_tokens` in Python |
| Joined components and type | `tag()` returns a dict and `Street Address` / `PO Box` / `Ambiguous`; raises `RepeatedLabelError` | None yet | `tag()`, `address_type()`; warning, not an error (decision 23) |
| Input forms | One string | One string or fields (`line1`, `line2`, city, state, ZIP) | Done |
| Default state | None | `default_state` hint, `--default-state` (decision 24) | Done |
| Normalized output | None (see `usaddress-scourgify`) | USPS Pub 28, RESO, key, slug | Done |
| Diagnostics, spans, confidence | None (CRF marginals only) | Warnings, unparsed text, byte spans, `place`; confidence planned (calibrated rule paths) | Milestone 7 |
| Rare address shapes | Basic | Rural and highway routes, military, general delivery, dual, second unit | Done |
| Customization | Retrain with parserator on your own labels | Fixed at compile time | Custom layer (decision 22, Milestone 9) |
| Intersections, landmarks, `CornerOf`, country, recipient names | Labeled | Not modeled; text goes to `unparsed` | Open decisions in design section 13 |
| Display casing, long form | Keeps input case | Uppercase only | Open decisions (Milestone 9, item 51) |
| Batch and dataframes | Loop yourself | `parse_many` (parallel with `rayon`) | Pandas/Polars extras (decision 26, Milestone 9) |
| Accuracy | Higher on suffix-less streets (84.8% vs 50.3% on the blind benchmark) | Higher with no city (99.0% vs 18.0%) | National place list (Milestone 5) |

**Customization without retraining.** usaddress adapts by labeling data and
retraining offline. This parser does not build a CRF (design.md decision 32):
the custom layer is the plan for adapting it, a few lines of aliases or
places, no retraining, and the change is visible in review.

## Label mapping

| This parser | usaddress label |
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

A `#` takes the label of the token after it, as usaddress's data does
(`# 234` is two `OccupancyIdentifier` tokens).

## Scoring rules

- Tokens are compared one by one; tokens with no letter or digit (`-`, `&`)
  are not scored.
- **Out of scope:** addresses whose gold labels include `Recipient`,
  `LandmarkName`, `CornerOf`, `IntersectionSeparator`,
  `NotAddress`, `CountryName`, `SubaddressType`, `SubaddressIdentifier`,
  `StreetNamePreModifier` or `AddressNumberPrefix` are left out; this parser
  does not model them.
- **Same thing, different label:** `AddressNumberSuffix` counts as
  `AddressNumber` on both sides. In raw mode, usaddress's `SubaddressType`
  and `SubaddressIdentifier` count as `OccupancyType` and
  `OccupancyIdentifier`.
- **Coarse score:** the five street-part labels fold into `StreetName`.
  `us50_test_tagged.xml` labels a whole street as `StreetName`
  (`Mendenhall Mall Road,`), so only the coarse score means anything there.
  The fine score is reported too.
- **Training overlap:** addresses listed in
  `tests/external/usaddress/in_training.txt` are reported apart.

## The catch: usaddress's own regression tests

usaddress's CI asserts the labels in these test files on every release, and
its comments say new formats go in both training and test data. So it
scores about 100% on them even where an address is not in its training
files. The gold numbers say how close this parser comes to usaddress's
behavior, not which parser is more accurate. A blind comparison needs labels
neither parser has seen (see "Next steps").

## Results (2026-10-15)

### usaddress's test files

Coarse score, addresses not in usaddress's training files. "Before" is the
first run; "after" follows the rule changes listed under "Gaps found".

| File | Addresses | Ours: whole address, before | Ours: whole address, after | usaddress |
|---|---|---|---|---|
| `labeled.xml` (hard cases) | 109, then 118 | 73.4% | 83.1% | 100.0% |
| `synthetic_clean_osm_data.xml` | 784 | 100.0% | 100.0% | 100.0% |
| `synthetic_osm_data.xml` | 786 | 99.7% | 99.7% | 100.0% |
| `us50_test_tagged.xml` | 555 | 99.1% | 99.5% | 100.0% |
| **All** | 2,239, then 2,248 | 98.4% | 98.9% | 100.0% |

Rerun on 2026-10-16, after the national place list and the Florida work
(Milestone 5, item 29), same files and scoring:

| File | Addresses | Ours: whole address | usaddress |
|---|---|---|---|
| `labeled.xml` (hard cases) | 118 | 89.8% | 100.0% |
| `simple_address_patterns.xml` | 5 | 100.0% | 100.0% |
| `synthetic_clean_osm_data.xml` | 784 | 100.0% | 100.0% |
| `synthetic_osm_data.xml` | 786 | 99.7% | 100.0% |
| `us50_test_tagged.xml` | 555 | 99.5% | 100.0% |
| **All** | 2,248 | 99.2% | 100.0% |

Tokens: ours 99.6%, usaddress 100.0%. The gain over 98.9% is in `labeled.xml`
(83.1% to 89.8%), the hard cases; the rest was already near its ceiling.
Fine score, all files: ours 75.1%, usaddress 76.2%.
Tokens: ours 99.4% before and 99.5% after; usaddress 100.0%. Nine more
addresses are in scope after the change because `BuildingName` is now
modeled. Over all in-scope addresses (the `cargo test` figure): 99.76% of
47,351 tokens and 99.46% of 9,036 addresses.

### Geocode benchmark (blind)

The test split of the [geocoding address-parsing benchmark](https://github.com/zhengcongyin/geocoding-address-parsing-benchmark)
(30,622 addresses, one line, no commas, street addresses only). It states
no license, so it is kept in the gitignored `local-data/` directory with the
run commands and reports, and is never committed. Neither parser trained on
it.

| | Ours, before | Ours, after | usaddress 0.5.16 |
|---|---|---|---|
| Tokens (coarse) | 95.3% | 97.2% | 97.7% |
| Whole address (coarse) | 76.1% | 85.4% | 88.2% |
| Whole address (fine) | 71.1% | 72.0% | 77.3% |

After the changes, both parsers are right on 23,587 addresses, only ours on
2,558, only usaddress on 3,413, and neither on 1,064. By shape: with a street
suffix, city, state and ZIP (19,507 addresses) ours is right on 92.4% and
usaddress on 96.4%; with no city, ours 99.0% and usaddress 18.0% (usaddress
makes a city out of street words); with no street suffix, ours 50.3% (up from
4.8%) and usaddress 84.8%.

What is left is mostly where the city starts on one line with no commas:
the city begins with a word left in a suffix-less street (6.4% of
addresses), a suffix word (3.3%) or a directional (2.5%). A place list fixes
these; the Census 2024 Gazetteer (places and county subdivisions, public
domain) covers 73.5% of the benchmark's cities.

After the national place list (Milestone 5, item 25), with the same
parsers and the same 30,622 addresses:

| | Ours, before the list | Ours, with the list | usaddress 0.5.16 |
|---|---|---|---|
| Tokens (coarse) | 97.2% | 99.2% | 97.7% |
| Whole address (coarse) | 85.7% | 96.1% | 88.2% |
| Tokens (fine) | 93.3% | 95.8% | 94.7% |
| Whole address (fine) | 72.3% | 82.0% | 77.3% |
| `PlaceName` tokens | 87.6% | 96.8% | 96.8% |

The list finds 87.4% of the benchmark's cities (`docs/places.md`). The
largest remaining error is still a city taken for street words (1,160
tokens, `281 EVELYN DR DOTHAN`: a city the list lacks or one that reads as a
street).

Rerun on 2026-10-16 for item 29, after the Florida items (26 to 28): the
numbers above did not change (96.1% whole address, coarse; 82.0% fine), so
those items did not touch this data. The errors that remain, by gold label
and what this parser predicted (tokens): `PlaceName` taken for `StreetName`
1,161 (`281 EVELYN DR DOTHAN`: no state and no ZIP, so no state to look the
city up in), `StreetName` taken for `PlaceName` 161 (`705 BLUE RIDGE VILLA
DR AL 35226`), `StateName` taken for `StreetName` 129 (`22 NEWMARKER SOUTH
WINDSOR CT 06074`: a city beginning with a state code), `StreetName` taken
for `OccupancyIdentifier` 83 (`143 LEE ROAD 2004 ...`, a county road name).
usaddress's largest error on the same data is `StreetName` taken for
`PlaceName`, 1,347 tokens.

### Raw files (no labels; agreement, coarse)

| File | Addresses | Same on every token | Tokens agreeing | Tokens ours left unplaced |
|---|---|---|---|---|
| Florida auction listings | 9,314 | 87.3% | 96.1% | 3.2% |
| DBPR licensees, region 4 (Florida) | 15,294 | 98.0% | 99.4% | 0.2% |
| DBPR licensees, region 11 (out of state) | 25,369 | 97.1% | 99.1% | 0.4% |
| usaddress `unparseable.csv` | 1,322 | 22.4% | 59.3% | 7.2% |

Rerun on 2026-10-16 (item 29), compared with the first run: the DBPR files
and `unparseable.csv` moved up a little, the auction listings did not. The
auction listings' disagreements are mostly addresses this parser leaves
unplaced on purpose (a landmark, `0 UNKNOWN`, `CONFIDENTIAL`).

Raw mode joins the columns into one line, so it does not exercise this
parser's multi-field rules. In hand-checked samples of disagreements, this
parser was right in about 19 of 25 auction rows, and usaddress in about 13
of 30 region 11 rows (ours in about 9). These samples are from the first
run.

## Gaps found (backlog)

Done on 2026-10-15 (corpus cases `rg-050` to `rg-081`; rules in
`docs/usps-pub28-notes.md`):

- A highway route ends the street in the city split; postdirectional after
  the route; name words after the route kept in every output.
- A unit absorbed by the city (`..., # 234 SANTA FE`); a unit before the
  street (`APT 1B 626 ...`); street and city in the first comma segment.
- HC with ROUTE; `BOX <number>` alone as a PO Box; military box line
  reversed.
- `HGWY`; `COUNTRY ROAD` before a number; hyphenated routes (`GA-25`).
- House numbers: `428 B SOUTH`, `T703`, `N165 W2123`; a stray backtick.
- Letter routes after a county prefix (`COUNTY ROAD J`).
- No-ZIP state at the end of one line; `missing_street_name`; floors as
  numbers; a `building` field.

Still open:

1. **City on one line with no comma** (`ST LOUIS`, `W. PALM BEACH`,
   `NORTH LITTLE ROCK`, `LA GRANGE PARK`, `ROARING BROOK TWP`): needs the
   place list (state packs).
2. **Named buildings without a building word** (`THE GARRETT`, `THE
   CHADWICK`, `UPTOWN SQUARE`, `FOREST COVE APARTMENTS`) stay unparsed.
3. **Highway modifiers** (`NORTH HIGHWAY 71 BUSINESS LOWELL`): BUSINESS stays
   in the name, so the city split can take it.
4. **Two unit numbers** (`STE 4 551`): left unparsed, as decided.
5. **Letters alone as house numbers** (`L HATHWAY AVENUE`): not taken, as
   decided.
6. **State-pack items:** SR, `PR 462`, Queens-style numbers, Puerto Rico
   urbanizations, `LEE ROAD 2004`-style county road names, `CTY ROAD`.

## Next steps

- Done: the place list and the full rerun with the CRF call (design.md items
  24 to 29): no CRF, decision 32.
- The geocode benchmark is the blind test; rerun it after each rule change
  (commands in the local, uncommitted `local-data/README.md`). A hand-labeled sample (100 to 200 rows) of the
  Florida files would add a blind test on the data this library targets
  (item 36).
