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
| `places_added.tsv` | 36 more communities, all `place` | `name`, `county`, `kind`, `addresses` (distinct street addresses seen) |
| `variants_added.tsv` | 37 more spellings of known places | `variant`, `place`, `addresses` |
| `zips_observed.tsv` | 341 more (ZIP, place) pairs: 259 ZIPs, 199 places | `zip`, `place`, `addresses` (distinct street addresses seen) |
| `overrides.toml` | 1 suffix spelling, 2 highway prefix spellings | `[[suffix]]` `spelling`, `suffix`; `[[highway]]` `spelling`, `highway`, `kind` |

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
- `overrides.toml` reads a suffix spelling as another suffix in Florida,
  and adds highway prefix spellings (below).

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
- **`SR` is STATE ROAD, `CTY HWY` is COUNTY HIGHWAY** (`overrides.toml`
  `[[highway]]`; item 26). Appendix F writes `SR 220` as `STATE ROAD 220`
  and `SR MM` as `STATE ROUTE MM`, so `SR` is left out of the base table. In
  the export 139 Florida street lines have `SR` before a route number
  (`22200 SR 60`, `177 N SR 51`) and none writes STATE ROUTE, so in Florida
  the tagger reads `SR 60` as a highway and the USPS output is `STATE ROAD
  60`. It is a listed variant, with no warning. Outside Florida `SR 60` stays
  a street name as written. `CTY HWY` (5 rows, `3709 CTY HWY 214`) is
  COUNTY HIGHWAY, raising `non_standard_abbreviation`. `CO RD`, `CO HWY` and
  `CNTY RD` were already county spellings in the base table. `CTY RD` has no
  row in the export (0 of 445,252) or the auction listings, so it is not
  added. Not covered: a letter route (`SR E 70`, 1 row) and `SR` run into
  its number (`SR200`). The tagger and the assembler ask `Lexicons::
  highway_in` with the address's state (its own, or the ZIP's). Corpus
  `fl-020` to `fl-023`.

- **Rarer misspellings and the penthouse shape** (item 27). These are
  general rules, not Florida ones, so they are in the base tables
  (`data/suffixes.toml`, `data/units.toml`, each with its evidence) and the
  tagger; the evidence is the DBPR export. `DRT` is DR (1 row; the same
  street in the same ZIP is written DR or DRIVE on 7 other lines), `LNE` is
  LANE (3 rows) and `SU` is SUITE (16 rows at 6 distinct addresses, `7925
  N WICKHAM RD SU B`), all observed spellings with
  `non_standard_abbreviation`. A penthouse right after a designator is one
  unit (decision 31; 162 rows): `APT PH 5213` is `APT PH5213`, `UNIT PH6`
  stays, and `SUITE PH` is `STE PH`. Before this a designator with no
  number and a second PH unit came out. A penthouse after a designator that
  has its own number (`APT 4 PH 5`) stays two units. Corpus `rg-098` to
  `rg-105`.

- **Communities and spellings added from the DBPR export** (item 28;
  `places_added.tsv`, `variants_added.tsv`; `tools/packs/measure_fl_places.py`).
  With the national list in (item 24), 2,957 of the export's 445,252 rows
  (2,631 distinct addresses, 849 cities) still had a city that matches no
  place. They were read for real communities and clear spellings of known
  places; a candidate needed at least 10 distinct street addresses and one
  main ZIP. `places_added.tsv` adds 19 communities the Census list lacks:
  `COCONUT GROVE` (142 addresses), `VIERA` (89), `HARMONY` (86), `CHAMPIONS
  GATE` (79, with `CHAMPIONSGATE`), `UNIVERSITY PARK` (77), `NORTH VENICE`
  (62), `HUTCHINSON ISLAND` (58), `SINGER ISLAND` (56), `REUNION` (43),
  `AMELIA ISLAND` (42), `BABCOCK RANCH` (42), `MIROMAR LAKES` (39),
  `SUGARLOAF KEY` (32), `KINDRED` (25), `WATERSOUND` (25), `LITTLE TORCH
  KEY` (20), `LAKE SUZY` (20), `RAMROD KEY` (18) and `SALT SPRINGS` (13).
  All are `place` rows. The county is GeoNames' county for the community's
  main ZIP, in the pack's spelling (`DADE`, `ST. LUCIE`). The ZIPs come
  from `observe_fl_zips.py`, rerun with the new places (`zips_observed.tsv`
  239 pairs, was 218). `variants_added.tsv` adds 16 spellings of places
  already known, each at least 10 addresses: `PT CHARLOTTE` (50), `STPETE
  BEACH` (47), `LAUDERDALE BY THE SE` (43), `MAIMI` (31), `WPB` (22), `PNTE
  VDRA BCH` (21), `NEW SMYRNA BC` (19), `PORT STLUCIE` (19), `FTLAUDERDALE`
  (18), `ST PETERSBURG BEACH` (16), `LA BELLE` (15), `NMB` (14), `FTMYERS`
  (12), `SANIBEL ISLAND` (11), `STJOHNS` (10) and `PT ORANGE` (10).
  `LA BELLE` needs the entry because the place comparison rule reads `LA`
  as the suffix LANE; changing that rule would merge `LADUE` and `LA DUE`
  in Missouri, two different places. `build.rs` takes both files add-only:
  a name or spelling whose key the pack already has is an error. A variant
  of a place only the national list has is added after it (and left out
  when `places-us` is off). Result: rows with a city that matches no place
  went from 2,957 to 1,348 (1,290 addresses, 804 cities). Corpus `fl-024`
  to `fl-027`; `fl-016` now uses `LAUDERDALE BY THE S`, because `LAUDERDALE
  BY THE SE` is a listed variant.

- **Three more DBPR licensee files** (2026-10-16; public record):
  `COSMETOLOGYLICENSE_1`, `CONSTRUCTIONLICENSE_1`, `lic03bb`, the same
  column layout as the earlier `RE_rgn` files (address in columns 5 to 10).
  613,719 distinct addresses, 570,078 in Florida. 99.0% of those with a city
  resolve to a place; the warnings were `zip_city_mismatch` 5,509,
  `unparsed_remainder` 5,274, `non_standard_abbreviation` 2,589,
  `missing_street_number` 2,018 and `possibly_truncated` 1,168. The parser
  fixes this found are in the changelog (a cut-off copy ahead of the
  address, designators joined to their values, a name on line 1, BAY and
  STUDIO units). The pack work these files led to is the next bullet.
- **Communities, spellings and ZIP pairs from five exports** (2026-10-16;
  `places_added.tsv`, `variants_added.tsv`, `zips_observed.tsv`). The two
  pack scripts now read every DBPR export together (`tools/packs/dbpr.py`:
  the parquet and the board `.csv` files), counting each address once:
  1,138,411 Florida rows, 961,275 distinct. Run
  `measure_fl_places.py` and `observe_fl_zips.py` with all five files
  (`dbpr_20251010_113032.parquet`, `RE_rgn4.csv`, `COSMETOLOGYLICENSE_1.csv`,
  `CONSTRUCTIONLICENSE_1.csv`, `lic03bb.csv`). Added: 17 communities
  (`MIDDLETON` 113 addresses, a Villages development in Lake County;
  `CUTLER RIDGE` 73; `PEMBROKE LAKES`; `CAROL CITY`; `DUETTE`; `DEER ISLAND`;
  `MAXVILLE`; `BONITA BEACH`; `BELLA COLLINA`; `CAPE HAZE`; `SEACREST`;
  `MATLACHA ISLES`; `WILBUR BY THE SEA`; `WEST HOLLYWOOD`; `HOLLYWOOD
  BEACH`; `SWITZERLAND`; `SAINT LUCIE WEST`), and 21 spellings of known
  places, each at least 10 addresses (`DORAL BRANCH` 34, `JAX` 32, `OVEIDO`
  26, `MERRIT ISLAND` 23, `PBG` 23, `STPETE` 21, `PSL` 20, `LEHIGH ACERS` 18,
  `PONTE VERDA BEACH` 17, `SPRING HILLS` 17, `KISSEMME` 14, ...). The
  observed-ZIP table is regenerated: 341 pairs (was 239; none dropped).
  With five times the data the "10 addresses" rule let in a nearby city
  written for a suburb (`ORLANDO` in Winter Garden's 34787: 23 of 4,435
  addresses), so that path now also needs 1% of the ZIP's addresses
  (`MIN_LARGE_SHARE`); 32 pairs it would otherwise have added stay out. On
  the three newer files `zip_city_mismatch` fell from 5,509 to 4,505, FL rows
  with a city that matches no place from 3,289 to 2,776. The rest of the
  mismatches are a long tail of about 2,900 pairs. Corpus `fl-028` to
  `fl-033`.

## To do

Still open (2026-10-16): Florida work found in the DBPR licensee export
(counts are rows out of 445,252 Florida addresses, after the 2026-10-16
fixes). This section keeps the evidence; the work is scheduled as items 26
to 28 in `docs/design.md` section 10 (all three done; what is left is below), and the decisions are in its section
13.

- **Unmatched cities still left** (after item 28): 1,348 rows. The largest
  are `MIDDLETON` (61 addresses, ZIP 34762; not identified as a community,
  so not added), `DEER ISLAND` (10), and communities just under the
  10-address line (`CAPE HAZE`, `MATLACHA ISLES`, `BELLA COLLINA`, `CAROL
  CITY`, `VILLAGE OF GOLF`, `WILBUR BY THE SEA`, `SEACREST`). The rest are
  one-off typos and abbreviations (`OVEIDO`, `JAX BEACH`, `PCB`). Rerun
  `measure_fl_places.py` to see the list. Related open decision in
  `docs/design.md` section 13: a city that is no known place, with a ZIP
  the pack knows.

## Versioning

`pack.toml` `version` is `YYYY.MM.N`. Bump it whenever the `.tsv` files
change. It is not reported in parser output yet (`docs/design.md` section 11).
