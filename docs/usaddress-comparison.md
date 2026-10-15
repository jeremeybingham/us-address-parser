# Comparison with usaddress

How this parser is compared with the Python
[usaddress](https://github.com/datamade/usaddress) library (design section 9
and Milestone 7, item 31), and the results.

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
| unparsed text, skipped copies | no label |

A `#` takes the label of the token after it, as usaddress's data does
(`# 234` is two `OccupancyIdentifier` tokens).

## Scoring rules

- Tokens are compared one by one; tokens with no letter or digit (`-`, `&`)
  are not scored.
- **Out of scope:** addresses whose gold labels include `Recipient`,
  `BuildingName`, `LandmarkName`, `CornerOf`, `IntersectionSeparator`,
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

Gold files, coarse score, addresses not in usaddress's training files:

| File | Addresses | Ours: tokens | usaddress: tokens | Ours: whole address | usaddress: whole address |
|---|---|---|---|---|---|
| `labeled.xml` (hard cases) | 109 | 91.0% | 100.0% | 73.4% | 100.0% |
| `simple_address_patterns.xml` | 5 | 100.0% | 100.0% | 100.0% | 100.0% |
| `synthetic_clean_osm_data.xml` | 784 | 100.0% | 100.0% | 100.0% | 100.0% |
| `synthetic_osm_data.xml` | 786 | 100.0% | 100.0% | 99.7% | 100.0% |
| `us50_test_tagged.xml` | 555 | 99.7% | 100.0% | 99.1% | 100.0% |
| **All** | 2,239 | 99.4% | 100.0% | 98.4% | 100.0% |

On the 6,788 addresses usaddress trained on: ours 99.8% of tokens and 99.6%
of addresses, usaddress 100.0% and 99.9%. 61 addresses are out of scope.
Over all in-scope addresses (the `cargo test` figure): 99.70% of 47,260
tokens and 99.29% of 9,027 addresses.

Raw files (no labels; agreement between the two parsers, coarse):

| File | Addresses | Same on every token | Tokens agreeing | Tokens ours left unplaced |
|---|---|---|---|---|
| Florida auction listings | 9,314 | 87.3% | 96.1% | 3.2% |
| DBPR licensees, region 4 (Florida) | 15,294 | 97.8% | 99.4% | 0.3% |
| DBPR licensees, region 11 (out of state) | 25,369 | 96.9% | 99.0% | 0.5% |
| usaddress `unparseable.csv` | 1,322 | 11.8% | 54.4% | 6.1% |

Raw mode joins the columns into one line, so it does not exercise this
parser's multi-field rules.

In samples of 25 to 30 disagreeing rows, checked by hand:

- **Auction file:** this parser was right in about 19 of 25. It flags
  placeholders (`NO SITUS`, `UNASSIGNED LOCATION RE`) and drops repeated
  ZIPs, where usaddress labels them as streets or units.
- **Region 11:** usaddress was right in about 13 of 30, this parser in about
  9, and neither in the rest.

## Gaps found (backlog)

From the 36 not-in-training gold addresses only usaddress gets right, plus
the raw samples:

1. **City on one line with no comma**, when it starts with a directional or
   holds suffix words: `... DR CHICAGO, IL`, `... PL ST LOUIS, MO`, `EL
   MIRAGE`, `W. PALM BEACH`, `NORTH LITTLE ROCK`, `LA GRANGE PARK`,
   `SPRINGBROOK TOWNSHIP`, `EAST PROVIDENCE RHODE ISLAND`. A place list (the
   state-pack plan) is the real fix.
2. **Unit absorbed by the city:** `1000 Cordova Place, # 234 Santa Fe NM`.
3. **Highway contract spellings with ROUTE:** `HC ROUTE 72 BOX 1A`,
   `HC R 32 BOX # E3`, `HIGHWAY CONTRACT RTE # 46`, `HWY CONTRACT ROUTE 102`.
   Pub 28 section 251 says to drop ROUTE.
4. **A bare box line:** `BOX 123456`, `BOX # 123456`, `ROUTE BOX # 65`.
5. **Highway variants:** `FM 544` (with a city after it), `PR 462` (Texas
   private road), `COUNTRY ROAD` (typo), `HGWY`, `NORTH HIGHWAY 71 BUSINESS`.
6. **Unit before the street:** `APT 1B 626 E KILBOURN AVE`.
7. **Unusual house numbers:** a grid number with a space (`N165 W2123`),
   letters (`L HATHWAY AVENUE`, `T703 STATE ROUTE 66`), a letter after the
   number (`428 B SOUTH MAIN ST`), a stray backtick (`1719\``).
8. **Two unit numbers** (`STE 4 551`), and building names (`EAST TOWER`),
   which are out of scope today.
9. **Military box with parts reversed:** `BOX 2877 PSC 103`.

## Next steps

- Work through the backlog, with a golden-corpus case per fix.
- A blind comparison needs labels neither parser has seen: hand-label a
  random sample (100 to 200 rows) of the Florida files, or use the test split
  of the [geocoding address-parsing benchmark](https://github.com/zhengcongyin/geocoding-address-parsing-benchmark)
  locally (it has no license, so it must not be committed).
