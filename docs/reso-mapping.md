# RESO field mapping (Data Dictionary 2.0)

Implemented by `crates/addr-core/src/emit/reso.rs` (`ParsedAddress::to_reso`).

This maps a parse to RESO field names and shapes. It does not validate
deliverability, and it does not check values against any server's lookups.

## Source and version

- **RESO Data Dictionary 2.0**, Property resource, group *Location / Address*:
  `https://dd.reso.org/DD2.0/Property/<FieldName>/`. The site footer reads
  "Generated October 12, 2026"; the pages were read on 2026-10-15.
- Lookups: `https://dd.reso.org/DD2.0/lookups/StreetDirection/`,
  `.../StreetSuffix/`, `.../StateOrProvince/`, `.../Country/`.
- Release overview: `https://dd.reso.org/DD2.0/about/release-guide/`.
- The Data Dictionary calls its maximum lengths "Suggested". The emitter never
  cuts a value; `ResoAddress::length_overruns()` reports the ones that exceed.

The emitter targets the fixed set in the table below. A newer Data Dictionary
(2.1 is a draft) needs this file rechecked, and a change of target is a
change to this document and to `reso.rs` together.

## Field table

Types, lengths and lookup status are as the Data Dictionary pages give them.

| DD field | DD type | Max len | Lookup | Emitted from | Notes |
|---|---|---|---|---|---|
| `StreetNumber` | String | 25 | | `street_number` | DD: may hold non-numeric characters and modifiers such as `1/2` or `B`; must not hold prefixes, direction or suffix. Same cleaning as the USPS line (uppercase; punctuation dropped except `-`, `/`, and `.` between digits). |
| `StreetNumberNumeric` | Number | 10 | | leading digits of `street_number` | DD: "the integer portion of the street number". `123-125`, `12A` and `123 1/2` give 123, 12, 123. Empty when the number starts with no digit or has more than 10 digits. A derived value. |
| `StreetDirPrefix` | String List, Single | 15 | `StreetDirection`, open with enumerations | `street_dir_prefix` | Lookup values are `E N NE NW S SE SW W`, the Pub 28 abbreviations. |
| `StreetName` | String | 50 | | `street_name`, or a folded form (below) | |
| `StreetSuffix` | String List, Single | 25 | `StreetSuffix`, open, **no standard values** | `street_suffix` abbreviation | See decision 1. |
| `StreetDirSuffix` | String List, Single | 15 | `StreetDirection` | `street_dir_suffix` | As the prefix. |
| `UnitNumber` | String | 25 | | `unit_type` + `unit_number`, then the second unit | DD definition: the unit "that should appear following the street suffix or, if it exists, the street suffix direction (e.g., Apt G, #55)". The field holds the designator with the number. See decision 3. |
| `City` | String List, Single | 50 | `City`, open | `city`, or the military post office | |
| `StateOrProvince` | String List, Single | 2 | `StateOrProvince`, open with enumerations | `state` code | See decision 4. |
| `PostalCode` | String | 10 | | `zip5` | |
| `PostalCodePlus4` | String | 4 | | `zip4` | DD: "the last four digits of a nine-digit U.S. ZIP Code". |
| `UnparsedAddress` | String | 255 | | the USPS line 1, city, state and ZIP | DD: the full civic location as one entity, optionally with City, StateOrProvince, PostalCode and Country. See decision 5. |

## Shapes with no field of their own (design decision 15)

The Data Dictionary has no field for these, so they are folded into
`StreetName`, with `StreetNumber` empty (except highways):

| Shape | `StreetName` | Other fields |
|---|---|---|
| Highway street (`100 KENTUCKY STATE HIGHWAY 625`) | state code, standard prefix, route: `KY STATE HIGHWAY 625` | `StreetNumber` kept; suffix and directionals as parsed |
| PO Box | `PO BOX 12` | |
| Rural route / highway contract | `RR 2 BOX 152`, `HC 68 BOX 23A` | |
| Military or State Dept. box | `PSC 1650 BOX 10` | |
| General delivery | `GENERAL DELIVERY` | |
| APO / FPO / DPO | not applicable | `City` = `APO`, `FPO` or `DPO`; `StateOrProvince` = `AA`, `AE` or `AP` |
| Second unit designator | not applicable | appended to `UnitNumber`: `APT 4 LOT 18` |
| Dual address: street and PO Box (Pub 28 section 215) | the street's name; the box is not emitted | street fields and units from the street; `UnparsedAddress` uses the street line (decision 8) |
| Attention or care-of line (`ATTN ...`, `C/O ...`, section 214) | not applicable | not emitted |
| Building name (`EAST TOWER`) | not applicable | not emitted |

The text is the same as the USPS emitter's delivery line (Pub 28 sections
237, 238.1, 239, 241 to 261), so the two outputs agree. The PO Box and route
box numbers could instead go in `StreetAdditionalInfo` (decision 2).

## Fields not emitted

| DD field | Why |
|---|---|
| `Country` | The parser reads no country, so emitting `US` would be an assumption of the library's scope, not a parse result. Decision 6. |
| `PostalCity` | DD: the official USPS city. That needs USPS data; this library does not validate or look up. |
| `StreetSuffixModifier` | DD: a suffix not in the pick list, or one that extends or prefixes the suffix. The parser either maps a suffix to Pub 28 (with `non_standard_abbreviation`) or leaves it unparsed. |
| `StreetAdditionalInfo` | DD: "information other than a prefix or suffix for the street portion of a postal address". Nothing in the parse maps to it unambiguously. Decision 2. |
| `CityRegion`, `StateRegion`, `CountryRegion` | Area fields (group Location / Area), not address parts. |

## Decisions

The Data Dictionary pages give definitions, types and lookups but not
conventions, so these are choices, not facts from the Data Dictionary. The
author reviewed all of them on 2026-10-15 and accepted the behavior as
implemented.

1. **`StreetSuffix` form.** The Pub 28 abbreviation (`ST`), matching
   `to_usps()`. The lookup is open and defines no values.
2. **PO Box, route and box numbers.** Stay folded into `StreetName`.
   `StreetAdditionalInfo` is not used.
3. **`#` units.** `UnitNumber` includes the designator and writes `#55`, as
   the Data Dictionary's own example does. The USPS line writes `# 55`
   (Pub 28 puts a space); the difference is intended.
4. **State codes outside the lookup.** The `StateOrProvince` lookup has 65
   values (50 states, DC, VI and 13 Canadian provinces and territories) and
   no `PR`, `GU`, `AS`, `MP`, `FM`, `MH`, `PW`, `AA`, `AE` or `AP`. The
   lookup is "open with enumerations", so the emitter passes Pub 28 codes
   through, with no warning. A server that enforces the enumerations would
   reject them.
5. **`UnparsedAddress` content.** Built as `LINE1, CITY, ST ZIP`, using the
   USPS line 1 and ZIP+4 with a hyphen. No country is added.
6. **`Country`.** Left empty. No option to set it.
7. **Text not placed.** `Diagnostics::unparsed` is not emitted in the RESO
   output; it stays available in the diagnostics.
8. **Dual addresses.** A record with a street address and a PO Box (Pub 28
   section 215) gives the street in every RESO field, because RESO address
   fields describe the property; the PO Box is left out. The USPS output
   writes the box instead (section 215 prefers it on the mailpiece), so the
   two outputs differ for these records. Decided by the author, 2026-10-15
   (the key uses the street too).

## Verification status

- Field names, types, lengths, lookups and the two example values were read
  from the pages listed above on 2026-10-15.
- Tests: `emit/reso.rs` unit tests and the `reso` expectations in
  `tests/corpus/reso.jsonl`. The corpus values were written from this table,
  not copied from emitter output.
- Not checked: behavior of a particular RESO server, and the Data Dictionary's
  Lookup Resource representation (a server may expose lookup values by
  `LookupKey` rather than the plain strings used here).
