# Prior art: the author's Florida address code

The author's auction-tracker repository handles Florida addresses. This
note records what it does, what carried over into this library, and what
is left for the state packs (`docs/design.md` section 11).

Reviewed 2026-10-14:

- **auction-tracker**: `packages/radar/src/radar/parse/addresses.py`,
  `parse/normalize.py` (`split_address`), `config/places.py`, and
  `tests/test_addresses.py`. It scrapes county foreclosure and tax-deed
  auction sites, so its input is messy assessor and clerk data.

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

### Lessons carried into the parser (Milestone 2)

All of these are implemented, with corpus cases in `auction-tracker.jsonl`
and rules in `docs/usps-pub28-notes.md` ("Parser rules"):

- **Trailing bare units** after a street type: `4 PORTOFINO DR 706`,
  `6380 CHASEWOOD DR B`, `3806 N OAK DR V 21`, `4100 N OCEAN DR C-22`. Not
  after HWY/HIGHWAY, and only when a real street name is left before the type.
- **Unit words glued to the street type**: `947 BIRD BAY CTAPT 107`,
  `2921 S ORLANDO DRUNIT 142`.
- **Other unit markers**: `-UNIT 15`, `#309`, `(110)`, and two in a row
  (`2613 LEAR RD -UNIT B LT 18`).
- **Glued house numbers**: `101BRADDOCK LN` is 101 + BRADDOCK LN; `42ND ST`
  is a street with no number.
- **Letter suffix on numbers**: `12A MAIN ST`, `12-A MAIN ST`.
- **A ZIP is not a unit**: `11176 W SAMPLE RD #11176` keeps 11176 as the unit.
- **Placeholders with a prefix or code**: `N/A - MAGNOLIA AVE`,
  `NO ADDRESS - 061S311301002007`.
- **Multi-field quirks**: `PUNTA GORDA, FL- 33983` (dash after the state),
  `ORLANDO, 32837` (no state), a garbled line 2
  (`NEW PORT RICHEY, FL- PORT RICHEY 34654`), line 1 ending with the ZIP
  (Escambia) or a cut-off copy of the city (Nassau:
  `85231 BOSTWICK DR FERNANDINA B`).

### Left for the state packs

- **Cities run into the street line**, with or without a space:
  `13551 WILSON STREETDADE CITY`, `1155 TITUS AVE TITUSVILLE`. Only safe with
  a place list; a word-boundary check stops `GRAND DRIVE` being read as
  `...RIVERVIEW`.
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
  belongs in pack-level diagnostics, never in the key (design section 6).
- `cities_fl.json` is the only source of Florida variants (an older copy
  of the list elsewhere is redundant). Its variants include abbreviations
  (`ALTAMONTE SPGS`), the state run into the name (`BOCA RATON FL`) and
  real typos (`ADVENTURA`). `SPGS`, `BCH`, `PRK` are Pub 28 suffix
  abbreviations used inside names, which the pack can expand from the
  suffix table.
- `SR` is State Road in Florida; `CR` is County Road. Kept out of the base
  highway table where they are ambiguous (`special.toml` comments).

### Corpus

`tests/test_addresses.py` has 33 labeled street lines,
16 placeholders and a dozen full cases; all are in
`auction-tracker.jsonl` except the ones that need a place list. The raw
address lines in the golden CSVs under `tests/golden/v2/` (305 rows from 8
counties) seeded `auction-tracker-golden.jsonl`; those CSVs hold the old
parser's output, so the labels were written by hand.

Open finding from that pass: `WINGATE ST 32507` parses as a street plus
ZIP, but its USPS output (`WINGATE ST` / `32507`) re-read with a comma reads
`WINGATE ST` as a city (`PALM BEACH, 32507` has the same shape). Only a place
list separates them, so the case was left out of the corpus.
