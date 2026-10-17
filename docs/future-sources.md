# Future test and data sources

Status: researched 2026-10-17, **deferred** (design.md decision 39). Nothing
here was downloaded into `local-data/` or run through the parser. This is the
record of what was looked at, what each source could do for the library, and
the plan to pick it up when the parser is next allowed to change what an input
produces (the v2, `k2` key version; decision 34).

## Why deferred

- The accuracy measured so far is good for a first release (below), and the
  key format is a public contract from the first release. A parser change
  that alters a key after release needs `k2`, so further rule work waits for
  that version instead of landing as small key breaks.
- The sources that matter are large. The most convenient one (Netsyms) is
  about 7 GB to download and 40 GB extracted; the author does not have the
  disk for it now.
- Before the first release a parser fix may still change a key if it is a
  real bug (`CLAUDE.md`), logged in `docs/key-format.md`. This document does
  not stop that; it only means no further source-driven work is scheduled.

## What v1's numbers cover

| Measure | Result | Covers |
|---|---|---|
| Blind geocode benchmark, whole address (coarse) | 96.1% (usaddress 88.2%) | 30,622 street addresses, one line, no commas, uppercase, no units, one per state and ZIP pair |
| usaddress's own test files | 99.2% (usaddress 100.0%, its regression tests) | Its labeled shapes |
| `tag()` against `usaddress.tag()` | joined dicts identical on 99.3% of 8,103 addresses; types 99.8% | Auction, DBPR and corpus inputs |
| Golden corpus | 381 cases, all passing | Hand-labeled shapes, mostly Florida and Pub 28 |
| Fuzzing | about 900k runs, no panic | `parse` and `parse_fields` with every emitter |

Not measured, so not claimed: mixed-case and multi-line input from other
states, units outside Florida, PO Boxes and routes outside the corpus, and
confidence (still `None`; item 36 and Milestone 7). The place list finds
87.4% of the benchmark's cities (`docs/places.md`). The library formats and
parses; it does not check that an address exists or is deliverable.

## Sources looked at

| Source | What it is | Terms | Verdict |
|---|---|---|---|
| [NAD](https://www.transportation.gov/gis/national-address-database) (US DOT) | Address points aggregated from state and local data (v22, March 2026, per Netsyms; about 90M records at an earlier release). Flat table, schema based on NENA CLDXF, components already split (`Add_Number`, `StreetName`, `StN_PreDir`, `StN_PosTyp`, `Unit`, `Post_City`, `Inc_Muni`, `Zip_Code`, `State`, `County`). [Schema PDF](https://www.transportation.gov/sites/dot.gov/files/2023-07/NAD_Schema_202304.pdf). | Data.gov lists it as public domain; the disclaimer gives no accuracy warranty and says it is aggregated from state and local data. The disclaimer and schema pages returned HTTP 403 to our fetcher, so **read both in a browser before relying on them**. | Best label source: split components, nationwide, and the only one we could possibly commit samples of. |
| [OpenAddresses](https://github.com/openaddresses/openaddresses) | Per-source county, state and city data. CSV columns (from a 2017 mirror): `LON, LAT, NUMBER, STREET, UNIT, CITY, POSTCODE, DISTRICT, REGION, ID, HASH`. | Pipeline output is not relicensed; each source keeps its license (many need attribution, some share-alike). The `sources/` metadata is CC0. | Most diverse real formats, one source per jurisdiction. Local only. `CITY` is reportedly often missing. Access to the batch site not verified. |
| [Netsyms Address Database](https://netsyms.com/gis/addresses) | 2026-05 edition, over 175M addresses (US, Canada, Mexico), one SQLite table `addresses`: `zipcode, number, street, street2, city, state, plus4, country` plus `latitude, longitude, source` in the full file. Built from NAD and OpenAddresses, street names standardized to Pub 28, ZIPs corrected with a USPS ZIP+4 subscription. Full file about 7 GB download, 40 GB extracted; lite about 4.3 GB and 29 GB without coordinates and `source`. | No formal license. Says facts are public domain, asks that people link to the site instead of redistributing the file; some county data came from free web map viewers. Pay what you want, including $0. `/legal` has no dataset terms. | One file with NAD plus OpenAddresses and a per-row `source`. Local only, never committed. |
| Kaggle "List of Real USA Addresses" | 234 addresses (address, city, state, zip) from a [GitHub gist](https://gist.github.com/HeroicEric/1102788). ZIP stored as an integer, so leading zeros are lost (`Abington MA 2351`). License unknown. | Unknown | Skip. Too small to measure anything. At most one corpus case for the lost-leading-zero ZIP (`invalid_zip`). |

### The Netsyms sample (1,000 rows, 2024 sample file)

Downloaded for inspection only; SHA256
`aadde1016680f820943bc6c2fae7edfa360155b3f16e46d1e489e3741e8e2b4d`. Not kept in
the repository or in `local-data/`.

- 800 US rows (VA 200; WV, TX, TN, OH, MD, IN 100 each) and 200 Canadian.
- `source` names the contributing agency per row (`NAD Kaufman County, Texas`,
  `OA/columbiana`), so results can be broken down by jurisdiction.
- `street` holds direction, name and suffix together (`N BURKHART DR`,
  `1988 RUSTLE LN W`), uppercase. Only the joined street can be compared.
- `street2` is empty in 89.5% of rows and holds a bare value (`A`, `501`) with
  no designator, so unit type cannot be checked.
- `plus4` is empty in all 1,000 rows.
- Source errors to exclude from any score: four Ohio house numbers of `-9`,
  `MDW CREEK DR` (an abbreviated name), `ROBERT BOUL` (a truncated Canadian
  street), `3451/2` (a fraction with its space lost).
- About 50 of the 800 US rows are highways (`US ROUTE 30`, `CRAIN HWY`); no
  PO Boxes.

### Limits that apply to all of them

- **Street addresses only.** No PO Boxes, rural routes, military or general
  delivery, which stay covered by the corpus and the DBPR data.
- **Labels are not hand-verified.** A mismatch is a parser bug or a source
  label error; disagreements need review.
- **Circularity.** Netsyms standardizes streets to Pub 28, the spec the parser
  follows, so suffix and abbreviation cases are easy, and its ZIP and city
  values may carry USPS-derived corrections. Rendering each row in several
  styles (commas or none, abbreviated or full, mixed case) and keeping the
  geocode benchmark as the realistic check limits this.

## Plan for v2 (not scheduled)

1. **Policy.** Write down what may be committed. NAD samples, if the license
   check holds; nothing from Netsyms or OpenAddresses. Private data stays in
   `local-data/` (`CLAUDE.md`); only numbers and our own fixes ship.
2. **Acquire.** NAD direct is the license-clean choice and has the split
   labels. Netsyms full (for `source`) is the convenient alternative if disk
   allows, deleting the extracted file after sampling by state. A few raw
   OpenAddresses sources if the messy local formats are wanted (Netsyms
   normalizes them away). Record checksums in `local-data/README.md`.
3. **Profile before parsing.** Null rates, unit presence, number shapes and
   suffix coverage by state and `source`, to choose strata. Include rare
   shapes: units, fractions, highways, grid numbers (`N165 W2123`),
   Queens-style hyphenated numbers, Puerto Rico urbanizations.
4. **Harness** in `tools/compare/` beside `compare_usaddress.py`: render each
   row in several styles, run `addr`, compare component by component, report
   per state, per shape and per `source`, with usaddress as the baseline.
   Review about 200 disagreements by hand: parser bug or label error.
5. **Feed back** (v2, `k2`): parser bugs become corpus cases; observed
   `(state, ZIP, city)` counts fill place-list gaps, the way the DBPR export
   did for Florida; per-state road conventions (`SR`, `CR`, `FM`) say which
   state packs are worth building; rows for one point in different renderings
   check key stability.
6. **Optional:** a NAD sample as one more calibration source for the
   confidence score (item 38), with the hand-labeled Florida sample as the
   check and the geocode benchmark check-only.

Revisit earlier if real use shows a persistent class of misparses, or when
the author has the disk and wants the v2 work to start.
