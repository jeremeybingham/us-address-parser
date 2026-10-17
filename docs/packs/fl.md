# Florida pack (`crates/doorplate/data/packs/fl/`)

The first curated state pack. It gives the parser Florida's place names, the
spellings seen in real data and the city each ZIP belongs to. It is compiled in
with the `state-fl` cargo feature (on by default).

This data helps the parser find and name the city. It does not say an
address exists or can receive mail.

## Files

| File | Rows | Columns |
|---|---|---|
| `pack.toml` | | state, pack version, sources |
| `places.tsv` | 665 places (411 municipalities, 254 other places), 998 variants | `name`, `county`, `kind`, `sources`, `variants` |
| `zips.tsv` | 1,473 ZIPs (107 shared with a second place) | `zip`, `place` (the main place), `shared` |
| `places_added.tsv` | 36 more communities, all `place` | `name`, `county`, `kind`, `addresses` (distinct street addresses seen) |
| `variants_added.tsv` | 37 more spellings of known places | `variant`, `place`, `addresses` |
| `zips_observed.tsv` | 341 more (ZIP, place) pairs: 259 ZIPs, 199 places | `zip`, `place`, `addresses` (distinct street addresses seen) |
| `overrides.toml` | 1 suffix spelling, 2 highway prefix spellings | `[[suffix]]` `spelling`, `suffix`; `[[highway]]` `spelling`, `highway`, `kind` |

- `name` is the place in upper case with `SAINT` written out
  (`PORT SAINT LUCIE`) and no punctuation (`OPA LOCKA`).
- `county` is the county name; Miami-Dade is `DADE`.
- `kind` is `municipality` (an incorporated city, town or village) or
  `place` (a community or postal name with no city government).
- `variants` are other spellings seen in county listings and public address
  lists, mostly typos (`ADVENTURA` for AVENTURA). Some are the names of real
  places in other states (`VENTURA`, `STANFORD`, `RENTON`), so the parser
  trusts a variant only in a city field or when the ZIP agrees.
- `shared` lists places that also use a ZIP whose main place is another one
  (the ZIP straddles a boundary).
- `zips_observed.tsv` adds places to a ZIP's shared places, from real
  addresses. `build.rs` fails on a pair `zips.tsv` already has, a ZIP it does
  not have, or an unknown place.
- `overrides.toml` reads a suffix spelling as another suffix in Florida, and
  adds highway prefix spellings (below).

## Sources and licenses

| Data | Source | License |
|---|---|---|
| Place list, county, kind, variants, shared ZIPs | Curated by the authors from the sources below and from Florida county listings | MIT OR Apache-2.0 |
| Place names (`census` in `sources`) | US Census Bureau, 2024 Gazetteer Files, places, Florida | Public domain |
| ZIP to main place (`geonames` in `sources`) | GeoNames postal code export, `US.zip`, Florida rows, https://download.geonames.org/export/zip/ | CC BY 4.0, attribution in `NOTICE` |
| Observed ZIP places (`zips_observed.tsv`), added communities and spellings, the `CR` and `SR` overrides | A Florida public-record address export (2025-10-10, 445,252 Florida addresses) and four further public-record address files (961,275 distinct Florida addresses together). Only city, ZIP and the parsed street key were used; no names are kept | Florida public record (Chapter 119, Florida Statutes) |

`zips.tsv` is GeoNames data: each ZIP's main place is GeoNames' place name for
it, copied as is. The `shared` column was curated by the authors. In
`sources`, `original` marks the authors' first hand-made list and `observed`
a place first seen in county listings.

## Maintaining the pack

The files are edited by hand, and `build.rs` fails the build on a malformed or
contradictory row: a name or variant that leads to two places, a ZIP that is
the main ZIP of two places, a shared ZIP that is no place's main ZIP, or a
name that is not upper-case words. Additions are add-only (a name or spelling
whose key the pack already has is an error), and each names its evidence.

The rules used for additions, so that a new one is judged the same way:

- **A community** is added when at least 10 distinct street addresses use it
  as the city and it has one main ZIP. The county is GeoNames' county for that
  ZIP, in the pack's spelling (`DADE`, `ST. LUCIE`).
- **A spelling** of a known place is added when at least 10 distinct
  addresses use it. `LA BELLE` needs an entry because the place comparison
  rule reads `LA` as the suffix LANE; changing that rule would merge `LADUE`
  and `LA DUE` in Missouri, two different places.
- **An observed ZIP pair** (a place written with a ZIP `zips.tsv` does not give
  it) is kept when the city is a pack name or listed variant and either at
  least 10 distinct street addresses use it and they are at least 1% of the
  ZIP's addresses, or at least 3 that are at least 5% of the ZIP's addresses
  (small and PO Box ZIPs). Fewer stay out: they are mostly a nearby big city
  written for a suburb (`ORLANDO` in Saint Cloud's 34771), which the
  `zip_city_mismatch` warning should still flag.
- A variant that is the name of a different Census place is dropped:
  `BELLVIEW` is a Census place in Escambia County (ZIP 32526), not a spelling
  of BELLEVIEW (Marion County).

`pack.toml` `version` is `YYYY.MM.N`. Bump it whenever a `.tsv` file changes.
It is not reported in parser output.

## How the parser uses it

- **Finding the city on one line.** With the state (or, with none, the ZIP's
  state) known, the longest run of words before the state or ZIP that is a
  place becomes the city, preferring a place the ZIP belongs to
  (`123 MAIN ST WEST PALM BEACH FL 33401`). A variant counts there only when
  the ZIP agrees. Otherwise the suffix rule decides.
- **A city run into the street** (`WILSON STREETDADE CITY FL`): a word that is
  a street suffix followed by a place is split, when a state, ZIP or another
  segment follows the place.
- **A street before a ZIP**: `WINGATE ST, 32507` is a street, since
  `WINGATE ST` is no place; `PALM BEACH, 33480` is a city.
- **`ParsedAddress::place`**: the place the city names (`name`), a listed
  spelling (`variant`), a cut-off city that only one of the ZIP's places
  starts with (`completed`), or the ZIP's main place when there is no city
  (`zip`). `city` itself is never changed.
- **`zip_city_mismatch`**: the city is a known place, but the pack does not
  list the ZIP for it. GeoNames gives one place per ZIP, while USPS accepts
  several city names for many ZIPs, so this flags a disagreement with the
  pack, not a wrong address (`FORT MCCOY` with 32182). On 440,000 distinct
  Florida addresses it fires on about 0.4%, mostly a nearby big city written
  for a suburb.
- **`CR` is CIRCLE** (`overrides.toml`). The base table reads `CR` after a
  street name as CREEK (an older Appendix C1 transcription). In the Florida
  address data 333 street lines end a name in `CR`; for 277 the same street
  in the same ZIP is also written CIRCLE or CIR, for none CREEK or CRK, and
  the other 56 have no other spelling. So in Florida (the address's state, or
  with none its ZIP's) `CR` is CIR, still with `non_standard_abbreviation`.
  `CR` before a route number stays COUNTY ROAD.
- **`SR` is STATE ROAD, `CTY HWY` is COUNTY HIGHWAY** (`overrides.toml`
  `[[highway]]`). Appendix F writes `SR 220` as `STATE ROAD 220` and `SR MM` as
  `STATE ROUTE MM`, so `SR` is left out of the base table. In the Florida
  address data 139 street lines have `SR` before a route number
  (`22200 SR 60`, `177 N SR 51`) and none writes STATE ROUTE, so in Florida
  the tagger reads `SR 60` as a highway and the USPS output is
  `STATE ROAD 60`. It is a listed variant, with no warning. Outside Florida
  `SR 60` stays a street name as written. `CTY HWY` (5 rows,
  `3709 CTY HWY 214`) is COUNTY HIGHWAY, raising `non_standard_abbreviation`.
  Not covered: a letter route (`SR E 70`, 1 row) and `SR` run into its number
  (`SR200`). The tagger and the assembler ask `Lexicons::highway_in` with the
  address's state (its own, or the ZIP's).

Other rare spellings found in the same data are general rules and live in the
base tables with their evidence (`crates/doorplate/data/suffixes.toml`,
`crates/doorplate/data/units.toml`): `DRT` is DR, `LNE` is LANE, `SU` is SUITE,
all observed spellings that raise `non_standard_abbreviation`.

## Known gaps

- Cities that match no place: about 3,800 rows in 2,573 distinct cities among
  961,275 distinct Florida addresses. The largest are not communities:
  placeholders and counties written as the city (`FLORIDA`, `FL`,
  `MIAMI DADE`, `BROWARD`), and communities and spellings just under the
  10-address line.
- `zip_city_mismatch` has a long tail of about 2,900 (city, ZIP) pairs, mostly
  a nearby big city written for a suburb.
- A city that is no known place, with a ZIP the pack knows: `place` stays
  `None`.
