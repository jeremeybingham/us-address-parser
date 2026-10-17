# Canonical key and slug spec (version k1)

This page specifies the **key** (below) and the **slug** (last section).

The key is a **public contract from the first release on**. Stored keys must
keep meaning the same thing, so after release any change to what an input
produces needs a new version prefix (`k2:`) and an update here. Never edit
`k1` in place. **Until the first release (nothing is published yet) a parser
fix may change the key an input produces**; each such change is logged, dated,
in "Changes before the first release" below (design.md decision 34). The
format itself (segments, form letters, `k1n`) has not changed.

A key says two inputs name the same place as far as this library can tell. It
does not say the address exists or is deliverable (see README).

## Format

```
k1:S|<number>|<predir>|<name>|<suffix>|<postdir>|<state>|<zip5>[|<unit>|<unit2>]
k1:P|<box>|<state>|<zip5>
k1:R|<RR|HC>|<route>|<box>|<state>|<zip5>
k1:M|<kind>|<number>|<box>|<post office>|<state>|<zip5>
k1:G|<state>|<zip5>
```

- Prefix `k1:`, then a one-letter **form**, then segments separated by `|`.
  Every form has a fixed number of segments, and an empty value leaves an
  empty segment (`...||...`).
- Forms: `S` street, `P` PO Box, `R` rural or highway contract route, `M`
  military or Department of State line, `G` general delivery. Because the
  form is part of the key, a PO Box key can never equal a street key.
- `S`: `<name>` is the street name; for a street named after a highway it is
  the standard prefix plus the route (`COUNTY ROAD 23`), after the state code
  when the highway name holds a state (`KY STATE HIGHWAY 625`, Pub 28
  Appendix F), and followed by any name words after the route
  (`HIGHWAY 66 FRONTAGE`; until 2026-10-15 those words were dropped). `<suffix>` and the
  directionals are Pub 28 abbreviations.
- `M`: `<kind>` is CPR, OPC, PSC, UPR or UNIT; `<post office>` is APO, FPO or
  DPO (empty when absent).
- `<state>` is the USPS code. `<zip5>` is five digits.
- **Dual addresses:** a record with a street address and a PO Box (Pub 28
  section 215, `dual_address` warning) uses form `S`, from the street; the
  box is not in the key. Decided by the author, 2026-10-15. Before that rule
  such records kept whichever address came first and left the other
  unparsed, so their keys could be `P` or `S`.
- **Floors:** a floor unit's value is a number (`5TH FLOOR` gives `5`), so
  unit-level keys for ordinal floors changed on 2026-10-15.
- **Parser fixes of 2026-10-16** (DBPR data, corpus `rg-082` to `rg-097`):
  keys change where the old parse was wrong. Units written in two parts
  (`APT D 906`), misspelled designators (`APTO 417`), suffix spellings
  (`OAKES BLV`) and a dash before a unit were read into the street name and
  are now split out; streets cut across lines (`2781 NE` / `15 ST`) are
  joined. On about 445,000 Florida DBPR addresses, 1,385 property keys and
  2,055 unit-level keys changed.
- **`CR` as CIRCLE in Florida** (2026-10-16, Florida pack `overrides.toml`):
  a Florida street ending in `CR` keys with suffix `CIR`, not `CRK`. On the
  DBPR Florida addresses, 333 keys changed.
- **Parser fixes of 2026-10-17** (the blind Florida sample, corpus `rg-121`
  to `rg-136`): letter routes after a highway prefix (`HIGHWAY A1A`), `178ST`
  split into `178 ST`, `DR E 203` read as postdirectional E and unit 203, an
  ordinal floor before `#` and a number, `PH FLOOR`, copies of the city after
  a unit, and a period after an abbreviation as a word break (`ST.CLOUD`).
  On 961,275 distinct Florida licensee addresses, 1,691 property keys and
  1,865 unit-level keys changed; on 8,293 auction listings, 4 and 4.
- **Parser fixes of 2026-10-17, second batch** (the last two Florida sample
  errors, corpus `rg-137` to `rg-144`): a two-letter directional written as
  two words (`N W 27 AVE`, `MAIN ST S W`) is NW, SW, NE or SE; a second house
  number before a street (`1309 1329 E VINE ST`, `5505 5505 N MILITARY TRL`)
  is left unparsed instead of starting the street name; box words typed in
  front of a numbered street (`P O BOX 8491 N W 17TH ST`) are unparsed and
  the street is the address. On 961,275 distinct Florida licensee addresses,
  4,313 property keys and 4,313 unit-level keys changed (a sample of both
  kinds read as corrections); on 8,293 auction listings, 3 and 3. The blind
  geocode benchmark is unchanged (96.4%, no address got worse).
- **Not in the key:** city, ZIP+4, unit type, the attention line, the
  building name, anything in
  `diagnostics`.
  - City is noisy and the field most likely to be corrected later; leaving it
    out keeps stored keys stable (design section 6).
  - ZIP+4 varies within one building.

## Strong and partial keys

A key with both a state and a ZIP5 uses the `k1:` prefix. If either is
missing, the prefix is `k1n:` and the same body follows (`k1n:S|123||MAIN|ST||FL|`).
A partial key matches less reliably, and it can never equal a located key, so
the two kinds do not mix in a join by accident. `MissingState` or
`MissingZip` is also set in the diagnostics.

## Units

`KeyOptions { include_unit }`, default **`false`** (property level).

- `false`: units are not in the key; every unit in a building shares it.
- `true`: for form `S` only, two segments are appended: the first and second
  unit numbers. Unit **types are dropped**: `Apt 4`, `Unit 4` and `#4` all
  give `4`, so the design example `Apt 4` and `#4` matches. The cost is that
  `Apt 2` and `Fl 2` in one building collide at unit level.
- A unit-level key is the property key plus two segments. A unit-level key
  for an address with no unit still ends in `||`, so it differs from the
  property key and from any unit-bearing key.
- Other forms ignore the option.

## Normalization

Applied to every free-text segment (number, name, route, box, unit):

1. Uppercase; drop periods and apostrophes (`O'BRIEN` and `OBRIEN` match).
   Pub 28 lets punctuation be omitted (section 222) and lists periods and
   apostrophes for removal (section 354). Section 232 keeps the period in a
   name such as `39.2 RD`, and the USPS emitter does, but the key drops it,
   so `39.2 RD` and `392 RD` share a key.
2. Collapse whitespace to one space.
3. Percent-encode (`%XX`, uppercase hex, UTF-8 bytes) every character outside
   `A-Z 0-9 space / -`. This keeps `|` and `%` out of segments; `clean.rs`
   already folds accents, so it rarely applies.

Spelling variants (`North`/`N`, `Street`/`ST`, `Florida`/`FL`) are resolved by
the parser through the Pub 28 tables, not by the key. Sources for those
tables: `docs/usps-pub28-notes.md`. The key format itself (segments, form
letters, dropped unit type, `k1n`) is this library's own convention, not a
USPS rule.

## No key

`key()` returns `None` when there is nothing to key on: no street, PO Box,
route, military line or general delivery, or a placeholder such as `NO SITUS`
(`Placeholder` warning).

## Changes before the first release

What an input produces has changed whenever a parse was wrong or a rule was
added. The format did not change. Counts are keys changed on the Florida DBPR
addresses where measured. Inline notes above cover 2026-10-15 and the first
rows of 2026-10-16 (dual addresses, floors, the parser fixes `rg-082` to
`rg-097`, `CR`).

| Date | Change | Keys affected |
|---|---|---|
| 2026-10-16 | National place list read in every state (item 25): a city on a line with no commas is no longer part of the street name (`123 MAIN ST ST LOUIS MO`), so the name and suffix segments change for those inputs | Street segments of one-line addresses whose city the old parser left in the street |
| 2026-10-16 | Florida `SR` before a route number is a highway (item 26): `SR 51` becomes `STATE ROAD 51` in the name segment; `CTY HWY 214` becomes `COUNTY HIGHWAY 214` | 139 Florida DBPR addresses use `SR` |
| 2026-10-16 | `DRT` and `LNE` are suffixes `DR` and `LN` (item 27) | Streets that ended in them |
| 2026-10-16 | A penthouse after a designator is one unit value (`APT PH 5213` is unit `PH5213`; item 27) | Unit-level keys only |
| 2026-10-16 | A cut-off copy of the address ahead of the whole one is dropped (`624 E Coloni 624 E COLONIAL DR`) | About 1,785 of 613,719 addresses in three DBPR files |
| 2026-10-16 | A designator joined to its value is split (`APT.1285`, `STE.200`, `APT:201`, `APT3D`) and `BAY`, `STUDIO`, `BOOTH` with a value are units; `SUITE NO. 102` drops the `NO` | Unit-level keys only |
| 2026-10-16 | A name or building on line 1 with the street on line 2 is the attention line, so the street comes from line 2 | 629 addresses in the same files |
| 2026-10-16 | `default_state` (only when the option is set): a city found in the hinted state leaves the street name; the state-dependent readings (`CR`, `SR`) apply. The key still has no state | Only parsers given the option |
| 2026-10-16 | A trailing country (`USA`, `UNITED STATES`) is dropped before tagging (item 54), so the key is the one the address has without it; before, the country broke the city, state or ZIP segments | Addresses that end in a country name |
| 2026-10-16 | An intersection (`MAIN ST & 1ST AVE`) has no key and no slug (item 55); before, the `&` and both streets went into the street name | Addresses with two streets joined by `&`, `AND`, `@` or `AT` |
| 2026-10-17 | Four parser fixes found by the Netsyms comparison (docs/future-sources.md): a word that merely began with a suffix spelling and ended in a place name is no longer split (`CREEKWOOD`, `DALEWOOD`, `AVENIDA DE LA LUNA`, a city such as `PICO RIVERA`); a word after `#` or a letter range after a designator is a unit; a value after `#` or a designator is not split (`# 2WD`, `APT I4`); `U S ROUTE 23` keeps its house number whole; a postdirectional before a city starting with NE is kept; on one line a state code that is also a suffix (`CT`, `LA`, `WY`) confirmed by its ZIP and city is the state; `WASHINGTON DC` is city and state | Street, city, state and unit segments of those inputs (Connecticut, Louisiana, Wyoming and DC addresses written without commas, and units written after `#`) |

| 2026-10-17 | Spaced two-letter directionals (`N W`, `S E`) are joined; a second house number (`1309 1329 E VINE ST`) and box words before a numbered street (`P O BOX 8491 N W 17TH ST`) no longer go into the street name or make a box | Street segments of those inputs (about 0.45% of Florida licensee addresses) |

| 2026-10-17 | Pub 28 re-check (item 44): the first of two suffix words is part of the name and is written out (`GULL PT RD` is `GULL POINT`, `4TH AVE DR` is `4TH AVENUE`; section 234.2), and a highway route keeps up to two letters (`ROUTE 1150EE`) | Name segment of 958 of 440,288 distinct Florida licensee addresses (0.22%); the highway-route letters are rare |

The slug and `place` are not part of the key and change with these too.

## Slug

`slug()` is a URL-safe, human-readable label. Unlike the key it is **not a
contract**: it is not unique, not versioned, and may change when parsing or
formatting improves. Never store it as an identifier; use the key.

- Built from the Pub 28 style output (`to_usps()`): delivery line with its
  unit, city, state, ZIP5. ZIP+4 is left out. Unlike the key, the city and
  the unit type are included, because a slug is for people to read.
- Lowercase ASCII letters and digits; every other run of characters becomes
  one hyphen, with none at either end. Apostrophes are dropped (`O'BRIEN` ->
  `obrien`) and `#` reads as `unit`, so `#4` gives `unit-4` while `Apt 4`
  gives `apt-4`.
- Missing parts are left out (`123-main-st`). `None` for a placeholder, an
  intersection or an address with no lines.
- A dual address (street and PO Box) is built from the street line and its
  units, not the PO Box the USPS output writes, so the slug, key and RESO
  output all name the street (`9418-cuba-rd-hayes-va-23072`; `docs/design.md`
  section 13, decision 11).
- Example: `123 N. Main Street, Apt 4, Miami FL 33101` gives
  `123-n-main-st-apt-4-miami-fl-33101`.

## Tests

`tests/corpus/keys.jsonl` pins exact key and slug strings (`key`, `key_unit`
and `slug` fields); unit tests in `crates/doorplate/src/emit/key.rs` and
`slug.rs` cover the rules.
