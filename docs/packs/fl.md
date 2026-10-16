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
| `zips_observed.tsv` | 218 more (ZIP, place) pairs: 175 ZIPs, 123 places | `zip`, `place`, `addresses` (distinct street addresses seen) |
| `overrides.toml` | 1 suffix spelling | `[[suffix]]` `spelling`, `suffix` |

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
- `zips_observed.tsv` adds places to a ZIP's shared places, from real
  addresses (below). `build.rs` fails on a pair `zips.tsv` already has, a
  ZIP it does not have, or an unknown place.
- `overrides.toml` reads a suffix spelling as another suffix in Florida
  (below).

## Sources and licenses

| Data | Source | License |
|---|---|---|
| Place list, county, kind, variants, shared ZIPs | auction-tracker `packages/radar/src/radar/data/cities_fl.json`, the author's checked list built from the sources below and from county auction and license data | MIT OR Apache-2.0 (the author's) |
| Place names (`census` in `sources`) | US Census Bureau, 2024 Gazetteer Files, places, Florida (`census_fl_places_2024.txt` in auction-tracker) | Public domain |
| ZIP to main place (`geonames` in `sources`) | GeoNames postal code export, `US.zip`, Florida rows (`geonames_fl_zip.txt` in auction-tracker), https://download.geonames.org/export/zip/ | CC BY 4.0, attribution in `NOTICE` |

| Observed ZIP places (`zips_observed.tsv`), the `CR` override | Florida DBPR real estate licensee export, 2025-10-10 (`local-data/florida/dbpr_20251010_113032.parquet`, not committed): 445,252 Florida licensee addresses. Only city, ZIP and the parsed street key are used; names never leave the file | Florida public record (Chapter 119, Florida Statutes); approved as a source by the author, 2026-10-16 |

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

`zips_observed.tsv` is built from the DBPR export:

```sh
cargo build --release -p addr-cli
uv run --with pyarrow python tools/packs/observe_fl_zips.py \
    local-data/florida/dbpr_20251010_113032.parquet
```

It parses the Florida rows with `addr parse-file` and keeps a (ZIP, place)
pair that `zips.tsv` does not have when the city is a pack name or listed
variant and either at least 10 distinct street addresses use it, or at
least 3 that are at least 5% of the ZIP's addresses (small and PO Box ZIPs,
`MADEIRA BEACH` in 33738). Fewer stay out: they are mostly a nearby big
city written for a suburb (`ORLANDO` in Saint Cloud's 34771), which the
warning should still flag. Pairs are found against `zips.tsv`, not against
the warning, so a rerun gives the same table. Of 1,582 pairs the export has
that `zips.tsv` lacks, 218 pass, covering 18,446 addresses. The rerun after the
national list (item 24) added 22 pairs for Census places the pack lacked
(`TRINITY` in 34655, `AVE MARIA` in 34142, `TIERRA VERDE` in 33715, ...)
and `BELLVIEW` in 32526: only 3 of the ZIP's 363 addresses, but a Census place
whose own ZIP GeoNames names PENSACOLA, so the script keeps it by name
(`KNOWN` in `observe_fl_zips.py`).

`overrides.toml` is written by hand; each entry names its evidence.

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
  On the DBPR export's Florida addresses it fired on 23,878 rows (5.4%) with
  `zips.tsv` alone, mostly municipalities inside a ZIP whose GeoNames place
  is a neighbour (`CORAL GABLES` in 33134, `WESTON` in 33331); with
  `zips_observed.tsv` it fires on 1,871 (0.4%), corpus `fl-014` to `fl-016`.
- **`CR` is CIRCLE** (`overrides.toml`). The base table reads `CR` after a
  street name as CREEK (an older C1 transcription). In the DBPR export 333
  Florida street lines end a name in `CR`; for 277 the same street in the
  same ZIP is also written CIRCLE or CIR, for none CREEK or CRK, and the
  other 56 have no other spelling. So in Florida (the address's state, or
  with none its ZIP's) `CR` is CIR, still with `non_standard_abbreviation`.
  `CR` before a route number stays COUNTY ROAD. Corpus `fl-017` to
  `fl-019`.

## To do

Still open (2026-10-16): Florida work found in the DBPR licensee export
(counts are rows out of 445,252 Florida addresses, after the 2026-10-16
fixes). This section keeps the evidence; the work is scheduled as items 26
to 28 in `docs/design.md` section 10, and the decisions are in its section
13.

- **Unincorporated communities the pack does not know** (item 28, after the
  national list). 4,514 rows have a city that matches no place, mostly real
  communities with no city government: `TRINITY` (403), `AVE MARIA` (277),
  `COCONUT GROVE` (216), `CITRUS SPRINGS` (170), `TIERRA VERDE` (154),
  `VIERA` (102), `HARMONY` (101), `UNIVERSITY PARK` (84). Add them as
  `place` rows with their county and ZIPs (the rest are typos and cut-off
  names such as `MAIMI`, `PT CHARLOTTE`). Many are Census places (CDPs) that
  the national list (item 24) adds, so remeasure after it. Related open
  decision in `docs/design.md` section 13: a city that is no known place,
  with a ZIP the pack knows. The national list is now compiled under the
  pack (item 24): it adds 405 Census places (1,070 in all, including
  `TRINITY` and `CITRUS SPRINGS`; `docs/places.md`) with their county, and
  the observed-ZIP table was rerun with them (`TRINITY` and the others in
  `zips_observed.tsv`). Remeasure this list against the new total first.
- **Rarer misspellings and the penthouse shape** (item 27). Suffix typos
  `DRT` (1 row) and `LNE` (3), and the unit designator `SU` (16, `SU B`). A
  penthouse written after a designator (`APT PH 5213`, `UNIT PH6`, `SUITE
  PH`; 162 rows) gives a designator with no number plus a PH unit; it is to
  be read as one unit (`APT PH5213`; `docs/design.md` decision 31).
- **`SR` and county road names** (item 26). `SR` before a route number is
  State Road in Florida (`SR 54`; 143 rows), left out of the base tables
  because it is State Route elsewhere (Appendix F). County road spellings
  such as `CTY RD` and `CO RD` (19 rows) are not highway prefixes yet. Both
  belong in `overrides.toml` or the highway table, with evidence like the
  `CR` entry's.

## Versioning

`pack.toml` `version` is `YYYY.MM.N`. Bump it whenever the `.tsv` files
change. It is not reported in parser output yet (`docs/design.md` section 11).
