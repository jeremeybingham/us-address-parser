# Florida pack (`data/packs/fl/`)

The first curated state pack (`docs/design.md` section 11, Milestone 5). It
gives the parser Florida's place names, the spellings seen in real data and
the city each ZIP belongs to. It is compiled in with the `state-fl` cargo
feature (on by default).

This data helps the parser find and name the city. It does not say an
address exists or can receive mail.

## Files

| File | Rows | Columns |
|---|---|---|
| `pack.toml` | | state, pack version, sources |
| `places.tsv` | 665 places (411 municipalities, 254 other places), 998 variants | `name`, `county`, `kind`, `sources`, `variants` |
| `zips.tsv` | 1,473 ZIPs (107 shared with a second place) | `zip`, `place` (the main place), `shared` |

- `name` is the place as auction-tracker spells it: upper case, `SAINT`
  written out (`PORT SAINT LUCIE`), no punctuation (`OPA LOCKA`).
- `county` is the county name; Miami-Dade is `DADE`.
- `kind` is `municipality` (an incorporated city, town or village) or
  `place` (a community or postal name with no city government).
- `variants` are other spellings seen on county auction cards and license
  files, mostly typos (`ADVENTURA` for AVENTURA). Some are the names of
  real places in other states (`VENTURA`, `STANFORD`, `RENTON`), so the
  parser trusts a variant only in a city field or when the ZIP agrees.
- `shared` lists places that also use a ZIP whose main place is another
  one (the ZIP straddles a boundary).

## Sources and licenses

| Data | Source | License |
|---|---|---|
| Place list, county, kind, variants, shared ZIPs | auction-tracker `packages/radar/src/radar/data/cities_fl.json`, the author's checked list built from the sources below and from county auction and license data | MIT OR Apache-2.0 (the author's) |
| Place names (`census` in `sources`) | US Census Bureau, 2024 Gazetteer Files, places, Florida (`census_fl_places_2024.txt` in auction-tracker) | Public domain |
| ZIP to main place (`geonames` in `sources`) | GeoNames postal code export, `US.zip`, Florida rows (`geonames_fl_zip.txt` in auction-tracker), https://download.geonames.org/export/zip/ | CC BY 4.0, attribution in `NOTICE` |

`zips.tsv` is GeoNames data: each ZIP's main place is GeoNames' place name
for it, copied as is. The `shared` column is from auction-tracker.

The other `sources` values: `original` is the first hand-made list in
auction-tracker; `observed` marks a place first seen on auction cards.

## How it is built

```sh
python3 tools/packs/build_fl.py <auction-tracker>/packages/radar/src/radar/data
```

The script checks, and stops on, any of these:

- a ZIP that is the main ZIP of two places, or whose place differs from
  GeoNames, or a GeoNames ZIP with no place;
- a shared ZIP that is no place's main ZIP, or listed as both main and
  shared for one place;
- a place marked `census` that is not in the Gazetteer (compared on letters
  only; `EVERGLADES CITY` is the Census's `Everglades`);
- a name or variant that is not upper-case words.

It drops, and reports, a variant that is the name of a different Census
place: `BELLVIEW` was listed for BELLEVIEW (Marion County), but Bellview is
a Census place in Escambia County (ZIP 32526). The fix belongs in
auction-tracker's list too.

`build.rs` then compiles the files and fails the build on a name or variant
that leads to two places. A variant that the place comparison rule already
covers (`ALTAMONTE SPGS`, `N MIAMI`) is dropped there, since the name
matches it anyway.

## How the parser uses it

- **Finding the city on one line.** With the state (or, with none, the
  ZIP's state) known, the longest run of words before the state or ZIP that
  is a place becomes the city, preferring a place the ZIP belongs to
  (`123 MAIN ST WEST PALM BEACH FL 33401`). A variant counts there only when
  the ZIP agrees. Otherwise the suffix rule decides, as before.
- **A city run into the street** (`WILSON STREETDADE CITY FL`): a word that
  is a street suffix followed by a place is split, when a state, ZIP or
  another segment follows the place.
- **A street before a ZIP**: `WINGATE ST, 32507` is a street, since
  `WINGATE ST` is no place; `PALM BEACH, 33480` is a city.
- **`ParsedAddress::place`**: the place the city names (`name`), a listed
  spelling (`variant`), a cut-off city that only one of the ZIP's places
  starts with (`completed`), or the ZIP's main place when there is no city
  (`zip`). `city` itself is never changed.
- **`zip_city_mismatch`**: the city is a known place, but the pack does not
  list the ZIP for it. GeoNames gives one place per ZIP, while USPS accepts
  several city names for many ZIPs, so this flags a disagreement with the
  pack, not a wrong address (`FORT MCCOY` with 32182, corpus case `rg-041`).
  On auction-tracker's 276 golden rows with a city and ZIP it never fires.

## Versioning

`pack.toml` `version` is `YYYY.MM.N`. Bump it whenever the `.tsv` files
change. It is not reported in parser output yet (`docs/design.md` section 11).
