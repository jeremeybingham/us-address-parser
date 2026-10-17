# National place list (`crates/doorplate/data/places/`)

Layer 2 of the lexicon (`docs/design.md` section 11, Milestone 5): place
names and ZIP places for every state, DC and Puerto Rico, generated from
public data. It lets the parser find a city on a line with no commas, tell a
street from a city before a ZIP, and complete a cut-off city. A curated state
pack (`docs/packs/fl.md`) sits above it and wins for its state (decision 29).

This data helps the parser find and name the city. It does not say an
address exists or can receive mail.

## Files

| File | Rows | Columns |
|---|---|---|
| `places.toml` | | list version, states, sources |
| `places.tsv` | 50,375 places: 19,491 municipalities, 19,988 places, 10,896 subdivisions; 16,290 variants | `state`, `name`, `county`, `kind`, `sources`, `variants` |
| `zips.tsv` | 41,154 ZIPs | `zip`, `state`, `place` |

Same shape as a state pack (`docs/packs/fl.md`), with a `state` column
because it is one table for all 52 states, and no `shared` column, because
GeoNames gives each ZIP one place.

- `name` is upper-case words of letters and digits: accents folded
  (`ANASCO`), apostrophes and periods dropped (`OFALLON`), other
  punctuation a space (`WINSTON SALEM`), `ST`, `FT`, `MT` written out
  (`SAINT LOUIS`), as in the Florida pack.
- `kind` is `municipality` (a city, town, village or borough with its own
  government), `place` (a Census designated place, a Puerto Rico comunidad,
  or a postal name GeoNames has and the Census does not) or `subdivision`
  (a township and the like; below).
- `sources` is `census`, `geonames` or both.
- `variants` are other spellings of a name that is the same place: a
  township's bare name and `TWP` form (`ROARING BROOK TOWNSHIP` has
  `ROARING BROOK` and `ROARING BROOK TWP`), and the long name of a
  consolidated city-county (`ATHENS` has `ATHENS CLARKE COUNTY`). There are
  no misspellings in this list; those are curated (the Florida pack).
- `county` is the county name without `County`, `Parish` or `Municipio`
  (`Baltimore city` stays `BALTIMORE CITY`). It is for telling places apart
  and is not used by the parser yet.

## Sources and licenses

| Data | Source | License |
|---|---|---|
| Places, county subdivisions (`census`) | US Census Bureau 2024 Gazetteer Files: places (32,333), county subdivisions (36,421), counties (3,222), https://www2.census.gov/geo/docs/maps-data/data/gazetteer/2024_Gazetteer/ | Public domain |
| County of a place or subdivision | Census 2020 reference lists `national_place_by_county2020.txt` and `national_cousub2020.txt`, https://www2.census.gov/geo/docs/reference/codes2020/ | Public domain |
| ZIP to place, postal-only places, county of a postal-only place (`geonames`) | GeoNames postal codes `US.zip` (41,489 rows) and `PR.zip` (177), https://download.geonames.org/export/zip/, downloaded 2026-10-16 | CC BY 4.0, attribution in `NOTICE` |

The source files are in `local-data/places/` (not committed; URLs, dates and
checksums in `local-data/README.md` and `SHA256SUMS` there).

## How it is built

```sh
python3 tools/places/build_us.py            # reads local-data/places/
```

The output is deterministic (a second run is byte-identical). The builder
stops on a name that is not upper-case words, two names that the parser's
place comparison rule (`lexicon/place_key.rs`, ported into the script with
the suffix and directional tables) would treat as one in a state, a ZIP whose
place is not in `places.tsv`, a ZIP outside a place's state, and a state with
no places or no ZIPs.

## What the builder settles

- **Places.** Every Gazetteer place in the 50 states, DC and Puerto Rico.
  The type word is dropped (`Alachua city` is `ALACHUA`; `Milford city
  (balance)` is `MILFORD`; the part after a comma is dropped). A
  consolidated city-county is called by its city (`ATHENS`, `LOUISVILLE`,
  `LEXINGTON`), with the Census name as a variant. Massachusetts' `Amherst
  Town` is `AMHERST`, with `AMHERST TOWN` a variant.
- **Subdivisions.** Only active governments (Census `FUNCSTAT` A) whose type
  mail uses: townships, plantations, gores, grants, purchases and locations
  keep the type word in the name (`WASHINGTON TOWNSHIP`) and add the bare
  name and, for townships and plantations, `TWP` and `PLT` forms as variants;
  towns, cities, villages and boroughs that are not Census places (New
  England, New York, Wisconsin) take the bare name. Census county divisions
  (`CCD`), unorganized territories, precincts, districts, wards, barrios and
  numbered townships are left out (20,299 subdivisions in all). A bare name
  that a place already has, or that two subdivisions share, is not added as a
  variant (4,400 and 78 dropped).
- **One county for a place that spans several.** The 2020 list names every
  county a place touches but not how much of it lies in each. 1,301 places
  span more than one; each gets the county whose center (Gazetteer
  `INTPTLAT`/`INTPTLONG`) is nearest the place's own internal point.
  Connecticut's counties were replaced in 2022 and are missing from the 2024
  Gazetteer, so a Connecticut place that spans several takes the lowest
  county code. It is a tie-break for display, not a fact about the place.
- **A subdivision name used several times in a state** (`WASHINGTON
  TOWNSHIP` in 22 Pennsylvania counties) is one row with no county.
- **The 462 places newer than the 2020 county list** get a county only if
  GeoNames has a ZIP with the same name in the state; otherwise it is empty.
  A guess from the nearest county center would be wrong often enough to
  mislead. 1,503 rows have no county in all (those, and the shared township
  names).
- **GeoNames names.** Each ZIP's place is GeoNames' name for it, folded to
  the same spelling. A name the Census does not have becomes a `place` row
  (6,508 rows, `sources` = `geonames`: `BROOKLYN`, `WASHINGTON CROSSING`).
  Where a Census row and a GeoNames name are the same place by the
  comparison rule, the row gets both sources. 206 pairs differ only in
  spacing (`LAKEVIEW` and `LAKE VIEW`); the parser cannot tell them apart,
  so there is one row, spelled as the better kind of source has it
  (municipality over place over subdivision).
- **Puerto Rico is in this pass** (`PR.zip` is a separate GeoNames file whose
  county column is the municipio). The 513 GeoNames rows left out are the
  511 military ZIPs (`APO AE` and the like, no state) and 2 Marshall
  Islands ZIPs. Other territories have no Census places in these files.
  Puerto Rico urbanizations are not handled here (backlog).

## How it is compiled in

`build.rs` compiles `crates/doorplate/data/places/` when the `places-us` cargo feature is on
(it is by default), under the state packs (decision 29):

- A state with a curated pack keeps the pack's places and ZIPs. A national
  row whose comparison key the pack already has, as a name or a variant, is
  the pack's and is skipped; the national list adds the rest. For Florida
  that is 405 Census places the pack lacks (`TRINITY`, `CITRUS SPRINGS`,
  ... all CDPs), taking its places from 665 to 1,070. The Census's
  `Everglades` is renamed `EVERGLADES CITY` by the builder (`CENSUS_NAMES`
  in `build_us.py`), the pack's name, so it is the pack's place and not a
  second one. The pack's 1,473 ZIPs are untouched (GeoNames is the source of
  both, so the national ZIPs for Florida are the same pairs); the new places'
  ZIPs come from the observed-ZIP table, which was rerun with them
  (`docs/packs/fl.md`); `build.rs` adds those pairs once the national rows
  are in, and leaves them out when `places-us` is off.
- For every other state a name, variant or ZIP that would lead to two places
  is a build error, as in a pack. Names, counties and kinds are checked the
  same way.
- The parser reads the national layer for every state (design.md item 25):
  `Lexicons::place`, `zip_places`, `places` and `is_place` cover all 52. So
  the Florida behaviors work everywhere: a city on a line with no commas, a
  street versus a city before a ZIP, a city run into the street, a cut-off
  city completed from the ZIP, and the place from the ZIP.
  `zip_city_mismatch` stays limited to states with a curated pack
  (`Lexicons::has_pack`; decision 30), because the list names one place per
  ZIP, and its place equals the written city on only 75.8% of the
  benchmark's addresses.

The tables are packed records in `OUT_DIR`, read with `include_bytes!`
(`emit_places` in `build.rs` has the layout), not tables of `&str`. A
`&str` table costs a pointer and a relocation per string: the first version
added 13.4 MB to the `doorplate` binary. Packed, the national list adds **2.9
MB** (stripped release binary 1.57 MB without it, 4.49 MB with it; the
tables are 3.1 MB before linking: names 0.53, keys 0.87, index 0.68, rows
0.45, ZIPs 0.37, and 0.2 for the any-state name index that `is_place`
uses). `build.rs` takes about 2 seconds more.

## Coverage

Measured on the blind geocode benchmark (`local-data/geocode-benchmark/`,
26,745 addresses with a city and a state), by the place comparison rule:

| | Share |
|---|---|
| City is in the list for its state | 87.4% (Gazetteer places alone: 73.5%) |
| ZIP is in `zips.tsv` | 88.2% |
| ZIP's place equals the city | 75.8% |

The last row is why `zip_city_mismatch` stays limited to states with a
curated pack (decision 30): GeoNames names one place per ZIP, and many ZIPs
are used by several. The list contains 657 of the Florida pack's 665 places
(the pack wins for Florida and adds the rest: `BAREFOOT BAY`, `BRUCE`,
`ENTERPRISE`, `INLET BEACH`, `PENSACOLA BEACH`, `PERDIDO KEY`, `WALNUT HILL`,
`WEEKI WACHEE`).

Florida's `rg-030` (`BELLVIEW`, 32526) is the case that showed the gap the
national list opens: `BELLVIEW` is a Census place in Escambia County, but
GeoNames names ZIP 32526 `PENSACOLA`, so the city looked wrong for its ZIP.
`zips_observed.tsv` now lists `BELLVIEW` as a place of 32526.
