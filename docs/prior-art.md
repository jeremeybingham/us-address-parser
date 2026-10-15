# Prior art: the author's Florida address code

Two existing repositories handle Florida addresses. This note records what
they do, what carried over into this library, and what is left for later
milestones. Read it before designing the tagger (Milestone 2) or seeding the
corpus (Milestone 4).

Reviewed 2026-10-14:

- **auction-tracker**: `packages/radar/src/radar/parse/addresses.py`,
  `parse/normalize.py` (`split_address`), `config/places.py`, and
  `tests/test_addresses.py`. It scrapes county foreclosure and tax-deed
  auction sites, so its input is messy assessor and clerk data.
- **dbpr_fl**: `stats.py` (`load_city_master`, `normalize_cities`) and
  `cities_fl.json`. It analyzes Florida DBPR real estate license files and
  only normalizes city names.

## auction-tracker

### How it works

Address cleanup runs in two steps:

1. `split_address(lines)` splits the site's address lines into
   `address_line1`, `address_line2`, `city`, `state`, `zip`, plus an
   `address_flag` for the shape of the input (`one_line`, `no_street`,
   `no_state`, `three_lines`, `line2_unparsed`, `missing`, `unknown`).
2. `clean(row, county, places)` parses the street line with regexes
   (`parse_street`) and resolves the city against `cities_fl.json`. It
   returns parts plus two **status** columns that say how far to trust them:
   - `street_status`: `ok`, `no_number`, `placeholder`, `multiple`,
     `ambiguous`, `truncated`, `missing`
   - `city_status`: `ok`, `completed`, `from_zip`, `zip_mismatch`,
     `unknown`, `missing`

Nothing is thrown away: raw lines are kept next to the parsed parts, and a
Django "address quality" page counts rows by status with no re-parse.

### Lessons carried into Milestone 1

| Lesson | Where it landed |
|---|---|
| Statuses are what make bulk data usable: users filter and count by them. | `Warning` gained variants for each street status, and `Warning::code()` gives a stable snake_case name for a database column (`types.rs`). |
| Placeholder text ("NO SITUS", "NOT ASSIGNED", "0 UNKNOWN", "UNASSIGNED LOCATION RE", "-") is common in assessor data and must not be parsed as a street. | `data/special.toml` `[placeholder]`, `Lexicons::is_placeholder`, `Warning::Placeholder`. |
| House number 0 means "no number" (usually vacant land). | `Warning::ZeroStreetNumber`. |
| "5210/5212 BRISTO ST", "1204-1206 COLT ST": several addresses in one field. | `Warning::MultipleAddresses`. |
| "409742ND SQ": a number run into a numbered street can't be split reliably. | `Warning::AmbiguousStreetNumber`. |
| County sites cut lines at about 30 characters, mid-street or mid-unit. | `Warning::PossiblyTruncated`. |
| A unit word with nothing after it ("... DR #", "... TRL UNIT"). | `Warning::MissingUnitNumber`, plus `UnitType::range_required` from Pub 28 C2 so the tagger knows when a number is expected. |
| Real data uses spellings Pub 28 doesn't list: `TRC` (trace), `UN` (unit, Pasco), `LT` (lot). | `observed = [...]` lists in `suffixes.toml` / `units.toml`, resolved with `MatchKind::Observed` so the parser can raise `Warning::NonStandardAbbreviation`. Every other street type in its `STREET_TYPES` set (`TERR`, `TR`, `WY`, `AV`, ...) is already a Pub 28 C1 variant. |
| "8932 HWY 2301", "3911 HIGHWAY 97": the number after a highway word is the road's, not a unit. | `data/special.toml` `[[highway]]`, `Lexicons::highway`. |
| "2108 AVENUE G", "147 SE AVENUE A": a suffix word can start the street name, and the letter after it is the name, not a unit. | Recorded for the tagger (below); no table change needed. |

### Lessons for later milestones

Tagger (Milestone 2):

- **Trailing bare units.** After a street type (and optional postdirectional),
  a short token with a digit, or a single letter, is a unit: `4 PORTOFINO DR
  706`, `6380 CHASEWOOD DR B`, `1480 MALIBU CIR NE 111`, `3806 N OAK DR V 21`,
  `4100 N OCEAN DR C-22`, `610 OAKS PLANTATION DR A4-2`. Not after HWY/HIGHWAY,
  and only when a real street name is left before the type.
- **Unit words glued to the street type**: `947 BIRD BAY CTAPT 107`,
  `2921 S ORLANDO DRUNIT 142`.
- **Other unit markers**: `-UNIT 15`, `#309`, `(110)`, and two in a row
  (`2613 LEAR RD -UNIT B LT 18`).
- **Glued house numbers**: `101BRADDOCK LN` (Flagler) is 101 + BRADDOCK LN;
  but `42ND ST` is a street with no number.
- **Letter suffix on numbers**: `12A MAIN ST`, `12-A MAIN ST`.
- **A ZIP is not a unit**: `11176 W SAMPLE RD #11176` keeps 11176 as the unit;
  a trailing 5-digit run preceded by `#`, a digit or `-` is not a ZIP.
- **Cities run into the street line**, with or without a space:
  `13551 WILSON STREETDADE CITY`, `1155 TITUS AVE TITUSVILLE`. Only safe with
  a place list (state packs); a word-boundary check stops `GRAND DRIVE` being
  read as `...RIVERVIEW`.
- **"N/A - MAGNOLIA AVE"**: strip a leading placeholder prefix.
- **Placeholder with a trailing code**: `NO ADDRESS - 061S311301002007`.

Multi-field input (Milestone 2, `input.rs`):

- City lines come as `PUNTA GORDA, FL- 33983` (dash after the state) and
  `ORLANDO, 32837` (no state). Line 2 can be garbled:
  `NEW PORT RICHEY, FL- PORT RICHEY 34654`.
- Line 1 can end with the ZIP (Escambia) or a cut-off copy of the city
  (Nassau: `85231 BOSTWICK DR FERNANDINA B`).

State packs (deferred, `docs/design.md` section 11):

- `cities_fl.json` is a checked place list: each place has county, kind,
  main ZIPs, shared ZIPs, sources (census, GeoNames) and variant spellings.
  Loading rejects a name or variant that leads to two places and a ZIP that
  is the main ZIP of two places, so lookups never depend on file order. This
  is the model for `packs/fl/places.csv` and `zips.csv`.
- Place-name comparison ignores case, punctuation, spacing and
  `ST`/`FT`/`MT` vs `SAINT`/`FORT`/`MOUNT` (`place_key`).
- City completion: a city cut at the line limit (`WESLEY CHA`) is finished
  only when exactly one place in the county starts that way.
- City from ZIP when the site's city is not a place (`TAL`, `SAINT LUCIE
  COUNTY`), and a `zip_mismatch` status when city and ZIP disagree. That
  belongs in pack-level diagnostics, never in the key (design 6.4).
- `SR` is State Road in Florida; `CR` is County Road. Kept out of the base
  highway table where they are ambiguous (`special.toml` comments).

Corpus (Milestone 4): `tests/test_addresses.py` has 33 labeled street lines,
16 placeholders and a dozen full cases; all are in
`auction-tracker.jsonl` except the ones that need a place list. The raw
address lines in the golden CSVs under `tests/golden/v2/` (305 rows from 8
counties) seeded `auction-tracker-golden.jsonl`; those CSVs hold the old
parser's output, so the labels were written by hand.

Open finding from that pass: `WINGATE ST 32507` parses as a street plus
ZIP, but its USPS output (`WINGATE ST` / `32507`) re-read with a comma reads
`WINGATE ST` as a city (`PALM BEACH, 32507` has the same shape). Only a place
list separates them, so the case was left out of the corpus.

## dbpr_fl

`stats.py` keeps only rows with city, state, ZIP, county and address line 1,
and normalizes cities with an exact lookup from `cities_fl.json` (413
places, 291 with variants). It does not parse street addresses.

What it adds is the **variant list**: real misspellings from license files
(`ALTAMONTE SPGS`, `BOCA RATON FL`, `BOYNTON\xa0BEACH`, `BOCA RATÓN`, a
trailing comma). Lessons:

- Input cleaning (Milestone 2, `clean.rs`) must handle non-breaking spaces,
  trailing commas, doubled spaces and accented letters before lookup.
- City fields often carry the state (`BOCA RATON, FL`, `... FL.`): strip a
  trailing state when splitting multi-field input.
- `SPGS`, `BCH`, `PRK` show up inside place names. They are Pub 28 suffix
  abbreviations used as abbreviations of words in names, which a FL pack
  can expand from the same suffix table.
- The variant list is a ready source of city aliases for the FL pack. It
  overlaps with auction-tracker's `cities_fl.json` (same origin) but adds
  license-file misspellings; merge them when the pack is built.
