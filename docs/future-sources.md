# Future test and data sources

Status: researched 2026-10-17. **The Netsyms full file was run before the
first release** (design.md decision 40, reversing decision 39 for this source):
results below. NAD direct and OpenAddresses stay **deferred** to v2 (the `k2`
key version; decision 34). This is the record of what was looked at, what each
source could do for the library, what the Netsyms run found, and the plan for
the rest.

## Results of the Netsyms run (2026-10-17)

The full file (2026-05 edition, 176.5M rows: 146.6M US, 90.5M of them from NAD
and 56.1M from OpenAddresses, across 956 US sources) was extracted to
`local-data/netsyms-2026/` (SHA256 of the archive checked against the
published value). `tools/netsyms/sample.py` drew a 1.6M-row working sample (a
uniform draw of 1 in 500, about 300 rows from every state and source, and
oversamples of units, odd house numbers and highways); `profile.py` profiled
the full file; `compare.py` wrote each US row back as an address in four
styles, parsed it, and compared number, street, unit, city, state and ZIP with
the source's own columns. No row appears in the repository.

**Profile.** House numbers are plain digits in 98.4% of US rows (a `-`, a
letter, a fraction or a second word in the rest). A unit is present in 14.8%
of US rows, from under 1% (Rhode Island, Vermont) to 36.1% (Massachusetts)
and 66.8% (DC) by state, usually as `APT n`, `STE n` or `# n`. 91.4% of US
rows carry a ZIP+4. Streets run 2 to 3 words in 94.6% of rows.
Some sources are malformed: literal `<NULL>` text inside values, house numbers
with spaces or parentheses, unit columns holding two bare numbers. About 2% of
rows are such source quirks and are scored apart.

**Accuracy** on the uniform draw (217,906 US rows, 213,551 without source
quirks), whole address matching the source on all six fields, before and after
the fixes below:

| Style | Before | After |
|---|---|---|
| `123 MAIN ST APT 4, MIAMI, FL 33101` | 99.52% | 99.86% |
| pre-split fields | 99.54% | 99.81% |
| one line, no commas | 99.20% | 99.81% |

Over every row of the uniform draw, quirks included: 97.77% to 98.14%
(commas), 97.81% to 98.12% (fields), 97.45% to 98.09% (one line). The
blind geocode benchmark moved from 96.1% to 96.4% whole-address (token
accuracy 99.2% to 99.3%). Rows with units (clean units): 98.2% to 99.9%.

**What was found and fixed** (docs/usps-pub28-notes.md, `nat-001` to `nat-023`;
the key changes are logged in docs/key-format.md):

1. A word that began with a suffix spelling and ended in a place name was
   split: `PICO RIVERA` into `RIV ERA`, `CREEKWOOD`, `DALEWOOD`, `MEADOWPARK`,
   `AVENIDA DE LA LUNA`, `STRAND`. The glued-city split now needs a real
   anchor (it was written for Florida data; the national place list made
   nearly every word a candidate).
2. A word after `#` (`# PRIMARY`, `# REAR`) and a letter range (`APT A-D`) are
   units.
3. A value after `#` or a designator is not split (`# 2WD`, `APT I4`).
4. `U S ROUTE 23` kept its house number whole.
5. A postdirectional was silently dropped before a city starting with NE (`RD
   NE, NEW SALISBURY`): it looked like a cut-off copy of the city.
6. On one line with no commas, `CT`, `LA` and `WY` were read as suffixes
   instead of states, losing the city and state (Connecticut, Louisiana and
   Wyoming addresses). A ZIP of that state plus a place before the code now
   settles it; `12 SIMON CT 32505` stays a suffix.
7. `WASHINGTON DC` before a ZIP lost its city (the phrase is also a spelling
   of the state).

**Known and left alone.** Names that end in a number read as a street plus a
unit (`ZEHNER ROAD 10`: a bare number after a street is a unit by design);
`COUNTY ROUTE` and `US ROUTE` are street names, not highways; `ST JOHN, VI`
and `EAST BOSTON` (no territory places in the list, a neighborhood that is not
a place); `J E MCDONALD FARM RD` where `J` reads as a number suffix (the two
readings are about equally common in the data); Puerto Rico street names such
as `15 SO`. None is a regression.

**Not yet used.** The observed `(state, ZIP, city)` counts could fill gaps in
the place list the way the DBPR export did for Florida, and per-state road
conventions could rank the state packs worth building (plan steps 5 below).

## Why the rest is deferred

- The key format is a public contract from the first release. A parser change
  that alters a key after release needs `k2`, so rule work driven by NAD
  direct and OpenAddresses waits for that version instead of landing as small
  key breaks. (The Netsyms run, which includes both sources' rows, was done
  before release; decision 40.)
- NAD direct and raw OpenAddresses add split labels and messier local formats
  that Netsyms normalizes away, which is worth having but not needed for v1.
- Before the first release a parser fix may still change a key if it is a
  real bug (`CLAUDE.md`), logged in `docs/key-format.md`. This document does
  not stop that; it only means no further source-driven work is scheduled.

## What v1's numbers cover

| Measure | Result | Covers |
|---|---|---|
| Blind geocode benchmark, whole address (coarse) | 96.4% (usaddress 88.2%; 96.1% before the Netsyms fixes) | 30,622 street addresses, one line, no commas, uppercase, no units, one per state and ZIP pair |
| usaddress's own test files | 99.2% (usaddress 100.0%, its regression tests) | Its labeled shapes |
| `tag()` against `usaddress.tag()` | joined dicts identical on 99.3% of 8,103 addresses; types 99.8% | Auction, DBPR and corpus inputs |
| Netsyms sample, whole address on all six fields | 99.8% (commas, fields, one line; rows without source quirks) | 217,906 US rows from 956 sources, rendered in four styles; excludes malformed source rows (2%) |
| Golden corpus | 428 cases, all passing | Hand-labeled shapes, mostly Florida and Pub 28; `ns-` cases use invented addresses |
| Fuzzing | about 900k runs, no panic | `parse` and `parse_fields` with every emitter |

Not measured, so not claimed: real (not rendered) mixed-case and multi-line
input from other states, PO Boxes and routes outside the corpus, and
confidence (always `None` in the first release; decision 43). The Netsyms rows are
street addresses rendered from the source's fields, not messy human input. The place list finds
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
   row in several styles, run `doorplate`, compare component by component, report
   per state, per shape and per `source`, with usaddress as the baseline.
   Review about 200 disagreements by hand: parser bug or label error.
5. **Feed back** (v2, `k2`): parser bugs become corpus cases; observed
   `(state, ZIP, city)` counts fill place-list gaps, the way the DBPR export
   did for Florida; per-state road conventions (`SR`, `CR`, `FM`) say which
   state packs are worth building; rows for one point in different renderings
   check key stability.
6. **Optional:** a NAD sample as one more source for the deferred confidence
   score (backlog entry "Calibrated confidence", decision 43), with the
   hand-labeled Florida sample as the check and the geocode benchmark
   check-only.

Revisit earlier if real use shows a persistent class of misparses, or when
the author has the disk and wants the v2 work to start.
