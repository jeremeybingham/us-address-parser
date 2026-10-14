# USPS Pub 28 notes

Sources for the Pub 28 tables and rules this library implements. Every USPS
rule in code must trace to an entry here (see `CLAUDE.md`).

Publication: USPS Publication 28, *Postal Addressing Standards*,
<https://pe.usps.com/text/pub28/welcome.htm>. Table edition targeted: the
current online edition as of October 2026.

Formatting to Pub 28 is not validation. Nothing here checks that an address
exists or is deliverable.

## Lexicon tables (Milestone 1)

`data/*.toml` is compiled into static tables by `crates/addr-core/build.rs`
and read only through `Lexicons` (`crates/addr-core/src/lexicon.rs`).
`build.rs` rejects malformed tables: a spelling that maps to two entries, a
duplicate abbreviation or code, a form that is not a normalized key, or an
entry whose `forms` leave out its own name or abbreviation.

### Verification status

`pe.usps.com` could not be reached from the environment where the tables
were written (the proxy denied it, and web archives were blocked too). So
each table was first transcribed from independent open-source copies. In
October 2026 the Appendix C1 and C2 tables were checked against a copy of
the live HTML pages supplied by the maintainer. Later in October 2026 the
live pages were fetched directly (`pe.usps.com` redirects clients that
don't send a browser User-Agent to a "page not found" page). Appendix B,
Appendix F and the chapter 2 sections cited below were checked against
them. Before release (Milestone 7), re-check every table against the live
Pub 28 pages.

Chapter 2 page map (all under `https://pe.usps.com/text/pub28/`): 225
Military Addresses `28c2_010.htm`; 233 Directionals `28c2_014.htm`; 237
Highways `28c2_018.htm`; 24 Rural Route `28c2_021.htm` to `28c2_026.htm`
(241 to 246); 25 Highway Contract Route `28c2_027.htm` to `28c2_031.htm`;
26 General Delivery `28c2_032.htm` and `28c2_033.htm`; 28 Post Office Box
`28c2_036.htm` to `28c2_040.htm` (281 to 285). Appendix F is `28apf.htm`.

| Table | Pub 28 source | Cross-check | Status |
|---|---|---|---|
| `suffixes.toml` | Appendix C1, Street Suffix Abbreviations (<https://pe.usps.com/text/pub28/pub28apc_002.htm>) | Copy of the C1 HTML table (October 2026). First transcribed from usaddress-scourgify `STREET_TYPE_ABBREVIATIONS` and street-address gem 2.0.0 `STREET_TYPES` (both MIT). | **Verified**: 202 abbreviations and every C1 spelling. The check found COMMONS/CMNS and the spelling CAUSWA missing (added), confirmed CROSSROADS/XRDS and TRAILER/TRLR, and found 20 spellings in the open-source copies that C1 does not list (CRT, CK, CR, LA, TR, PK, ...); those are now `observed`. C1 quirks kept as is: it lists MDW under both MEADOW and MEADOWS (we map MDW to MEADOW only; see the table comment), and it labels the ANX row's primary name ANEX (we use ANNEX). |
| `units.toml` | Appendix C2, Secondary Unit Designators (<https://pe.usps.com/text/pub28/pub28apc_003.htm>) | Copy of the C2 HTML table (October 2026). First transcribed from usaddress-scourgify `OCCUPANCY_TYPE_ABBREVIATIONS`. | **Verified**: all 24 designators and abbreviations, and the `range_required` column (C2 marks BSMT, FRNT, LBBY, LOWR, OFC, PH, REAR, SIDE, UPPR as not needing a secondary range). C2 spells the HNGR description "Hanger"; HANGAR is kept as `observed`. C2 also has a "Blank, unable to determine" row that requires the pound sign (#) on the mailpiece; that is the `#` designator the tagger handles. |
| `states.toml` | Appendix B, Two-Letter State and Possession Abbreviations (<https://pe.usps.com/text/pub28/28apb.htm>); section 225.1 for AA/AE/AP | Live Appendix B table (October 2026). First transcribed from usaddress-scourgify `STATE_ABBREVIATIONS` and the street-address gem `STATE_CODES`. | **Verified**: all 62 codes and names, including the Military "State" table: AA Armed Forces Americas (except Canada), AE Armed Forces Europe, the Middle East, and Canada, AP Armed Forces Pacific. Section 225.1 adds Africa to AE; the AE aliases cover all four regions. |
| `directionals.toml` | Section 233 Directionals; Appendix B Geographic Directional table | Live section 233 and Appendix B (October 2026). | **Verified**: the eight directionals and abbreviations match Appendix B. Section 233.1 says to abbreviate them. Section 233 also has the parsing rules (NORTH-SOUTH and EAST-WEST pairs, a directional between name and suffix stays spelled out); those belong to the tagger. |
| `special.toml` `[po_box]` | Sections 281 Format, 282 Leading Zero, 283 Designations, 284 PO Box Street Addressing | Live sections 281 to 284 (October 2026). | **Verified**: standard `PO BOX ##`. Section 283 lists CALLER, FIRM CALLER, BIN, LOCKBOX and DRAWER to be changed to PO BOX (BIN was missing and is now added). PO DRAWER, LOCK BOX and the PO BOX spelling variants are not named in Pub 28; they stay as accepted input. |
| `special.toml` `[rural_route]`, `[box]` | Sections 241 Format, 242 Leading Zero, 243 Hyphens, 244 Designations RFD and RD, 245 Additional Designations | Live sections 241 to 245 (October 2026). | **Verified**: standard `RR ## BOX ##`. Section 241 says not to use RURAL, NUMBER, NO or the pound sign. Section 244 says to change RFD and RD (meaning rural) to RR, so RD is now a rural route form, and the tagger takes it only in the `RD ## BOX ##` position. RTE, RT and ROUTE are not named in 24x; they stay as accepted input. |
| `special.toml` `[general_delivery]` | Section 261 Format | Live section 261 (October 2026). | **Verified**: GENERAL DELIVERY is spelled out with no abbreviation, so the shortened forms are input only. |
| `special.toml` `[highway_contract]` | Section 251, Highway Contract Route Addresses: Format | Copy of the section 251 text and its example (October 2026). | **Verified**: the standard form is `HC ## BOX ##` (example: HC 68 BOX 23A). Section 251 names HIGHWAY CONTRACT and STAR ROUTE as words not to use, which confirms them as input-only variants. HCR, H C, HWY CONTRACT and STAR RTE are not named there; they stay as accepted input. |
| `special.toml` `[[highway]]` | Section 237 Highways (<https://pe.usps.com/text/pub28/28c2_018.htm>); Appendix F, Address Standardization: County, State, Local Highways (<https://pe.usps.com/text/pub28/28apf.htm>) | Live section 237 and Appendix F (October 2026). | **Verified**: the prefix is spelled out in the standard form: COUNTY ROAD, COUNTY HIGHWAY, HIGHWAY, STATE HIGHWAY, STATE ROAD, STATE ROUTE, US HIGHWAY, INTERSTATE, RANCH ROAD. FM is the exception (FARM TO MARKET 1200 becomes FM 1200). Added from Appendix F: the spellings ST HIGHWAY, ST RD and ST RT, and the prefixes TOWNSHIP ROAD (TSR), ROAD (RD), ROUTE (RT, RTE), EXPRESSWAY and LOOP. Appendix F maps SR to both STATE ROAD and STATE ROUTE, which confirms that SR has to be left to state packs. Each entry's comment lists the spellings Appendix F shows. |
| `special.toml` `[placeholder]` | Not a USPS rule | auction-tracker `PLACEHOLDER_RE` and its tests (see `docs/prior-art.md`). | Data-quality list; no USPS claim. |

### Non-Pub 28 spellings

Spellings that Pub 28 does not list are kept apart from
the Pub 28 forms, in `observed = [...]` lists with a source comment. Lookups
return them with `MatchKind::Observed` so the parser can report
`Warning::NonStandardAbbreviation`. Entries seen in field data, all from
auction-tracker (Florida county data):

| Spelling | Resolves to | Table |
|---|---|---|
| `TRC` | TRCE (TRACE) | suffixes |
| `UN` | UNIT | units |
| `LT` | LOT | units |

Also `observed`, but taken from older transcriptions of the Pub 28 tables
rather than from field data: 20 suffix spellings that usaddress-scourgify and
the street-address gem list but the current Appendix C1 does not (for example
CRT for COURT, CR and CK for CREEK, LA for LANE, TR for TRAIL), and HANGAR,
the dictionary spelling of C2's "Hanger". See the per-entry comments in
`data/suffixes.toml` and `data/units.toml`.

State `aliases` (FLA, CALIF, ...) are traditional, pre-ZIP abbreviations,
not Pub 28 forms. They are accepted as input (`MatchKind::Variant`) and never
emitted.

### Lookup rules

- Keys are normalized by `lexicon::normalize_key`: uppercase, periods
  removed, whitespace runs collapsed, trimmed. So "P.O. Box" looks up as
  "PO BOX" and "N.E." as "NE". Data files must already be written in this
  normalized form (`build.rs` checks).
- Several spellings belong to more than one table (FL is a state and the
  FLOOR designator; KEY is a suffix and a designator; CR is creek and county
  road; ST is street and saint; RD is the ROAD suffix, a rural route form and
  the ROAD highway prefix; ROUTE is a rural route form and a highway prefix). The lexicon answers each table on its own.
  Choosing between them by position is the tagger's job.
- State-dependent abbreviations (SR for State Road in Florida vs State Route
  elsewhere; RR for Ranch Road in Texas) are left out of the base tables, to
  be added by state packs.

## Parser rules (Milestone 2)

How the tagger (`crates/addr-core/src/tagger/`) reads an address, and where
each rule comes from. Rules from Pub 28 cite its section; the rest come from
shapes in real county data (`docs/prior-art.md`) and are not USPS rules.

| Rule | Source |
|---|---|
| Read the last line (ZIP, state, city) first, then the delivery line from both ends: units and postdirectional from the right, house number and predirectional from the left. | Section 231 (parse from the right-most element). |
| Directionals: one or two words before the name are the predirectional, after the name and suffix the postdirectional, abbreviated to the Appendix B forms. A directional with no name left beside it is the name (`100 N ST`, `2 SOUTH ST`). | Sections 233.1, 233.21, 233.22; Appendix B. |
| A one-letter directional right after a lone suffix word is part of the name, like any letter there (`AVENUE N`, `AVENUE G`). | Section 233.3 (letters used as alphabet indicators); prior art (`2108 AVENUE G`). |
| Suffix: the last word before units and postdirectional, when a name is left before it. Suffix words earlier in the line stay in the name (`EAGLES POINT CIRCLE`). | Section 234; Appendix C1. |
| Highway names: a prefix from `[[highway]]` right after the house number (or a predirectional) and followed by a route that starts with a digit. The prefix and route go in `highway`; later words are suffix, postdirectional or name (`HWY 11 BYPASS`, `US 41 SW`). `I10` and `IH280` are split by the tokenizer. | Section 237; Appendix F. |
| Units: a designator and value (`APT 4B`), `#` and value, a designator with `#` (`UNIT # 4`), or a designator alone (`REAR`, or `UNIT` with a missing number when Appendix C2 requires a range). Up to two, the first in input order going to `unit_*`. | Section 213; Appendix C2 (designators, range column, `#` when the designator is unknown). |
| Bare units: after a suffix (and optional postdirectional), a value with a digit or a single letter, or a letter and a number (`DR 706`, `DR B`, `DR V 21`). Not a directional letter (`PARK AVE S` is a postdirectional), not after a highway word other than ROAD, and only when a name comes before the suffix. | Prior art (Florida county data). |
| PO Box: a `[po_box]` form or synonym, then an optional `#` and the box value. | Sections 281, 283. |
| Rural route and highway contract route: the route word, optional `#`, route number, then `BOX` and the box value. RD, RT, RTE and ROUTE (also highway prefixes or suffixes) need the BOX part. | Sections 241, 244, 251, 253. |
| General delivery: the `[general_delivery]` forms. | Section 261. |
| Military: a `[[military]]` type, its number, then `BOX` and the box value; APO, FPO or DPO before AA, AE or AP takes the city's place. | Sections 225.1, 238.1, 239. |
| ZIP: the last token, 5 or 9 digits or `5-4`, not after `#`. A shorter all-digit token after a state is an invalid ZIP when the state starts its line or the input is at least 30 characters long. | ZIP formats; truncation from prior art. |
| State: right before the ZIP (traditional abbreviations such as FLA allowed), else first in the ZIP's line. With no ZIP, only a USPS code or full name ending a later line, or ending a one-line address after a city. | Appendix B; prior art (`FL- PORT RICHEY 34654`). |
| City: the line before the state or ZIP line, or the words before the state on its line. On a line with no commas, the words after the street's suffix (and any postdirectional and unit). A one-line address whose street has suffix words in the city (`MOUNTAIN VIEW`) needs a place list, which is a state-pack job. | Prior art; design section 11. |
| A copy of the city at the end of the street line, possibly cut off, is dropped (`... DR FERNANDINA B`). A suffix is never dropped this way. | Prior art (Nassau). |
| Placeholders: a whole street line in `[placeholder]`, or a placeholder before ` - ` (dropped, or the whole line when only a code follows). | Prior art; not a USPS rule. |
| Truncation: `possibly_truncated` when a line of at least 30 characters ends in something incomplete: a cut-off ZIP, a designator without its number, a cut-off designator (`UNI`), or a name with no suffix. | Prior art (county sites cut lines at about 30 characters). |
| The parser does not guess: `409742ND` (4097 42ND or 40974 2ND), slash pairs (`5210/5212`) and anything no rule claims go to `unparsed`. | Prior art. |

## Emitter rules

TODO: written in Milestone 3. Rules already sourced:

- Highway contract route (section 251): emit `HC <route> BOX <box>`, e.g.
  `HC 68 BOX 23A`. Drop HIGHWAY CONTRACT, ROUTE, NUMBER, NO, STAR ROUTE and
  the pound sign, so `HIGHWAY CONTRACT ROUTE 68 BOX 23A`,
  `STAR ROUTE 68 BOX 23A` and `HC RTE #68 BOX 23A` all become
  `HC 68 BOX 23A`. The route and box values may hold letters (23A).
- Rural route (sections 241 to 245): emit `RR <route> BOX <box>`. Drop
  RURAL, NUMBER, NO and the pound sign; RFD and RD become RR (244). A
  leading zero on the route number is not needed (242). Keep hyphens in the
  box number (243 keeps them only when the ZIP+4 file has them; without the
  file, leave them as given). No street or town name on the RR line (245).
- PO Box (sections 281 to 283): emit `PO BOX <number>`. CALLER, FIRM CALLER,
  BIN, LOCKBOX and DRAWER become PO BOX (283). The box value may be a letter.
  Leading zeros in the box number are kept (282: the ZIP+4 file marks
  significant leading zeros, which are printed as zeros). Section 284: an
  address using the Post Office's street address with `# <box>` or
  `UNIT <box>` is a street address with a secondary unit, not a PO Box line.
- General delivery (section 261): emit `GENERAL DELIVERY`, spelled out.
- Highways (section 237, Appendix F): emit `<standard prefix> <route>`, for
  example `CR 1185` becomes `COUNTY ROAD 1185`, `HWY FM 1320` becomes
  `FM 1320`, `I10` becomes `INTERSTATE 10`, `US 41 SW` becomes
  `US HIGHWAY 41 SW`. Routes may hold letters (`COUNTY HIGHWAY 60E`,
  `STATE ROUTE MM`). After the route number, Appendix F abbreviates only the
  last word, as a normal suffix, and spells earlier ones out:
  `HWY 11 BYPASS` becomes `HIGHWAY 11 BYP`, `HIGHWAY 3 BYP ROAD` becomes
  `HIGHWAY 3 BYPASS RD`, and `HWY 66 FRONTAGE ROAD` becomes
  `HIGHWAY 66 FRONTAGE RD`. A state name inside the street name is written
  as its two-letter code (`KENTUCKY STATE HIGHWAY 625` becomes
  `KY STATE HIGHWAY 625`, and a bare `KENTUCKY 440` or `KY 1207` becomes
  `KY HIGHWAY 440` or `KY HIGHWAY 1207`), but a state name that is the
  whole primary name stays spelled out (`OKLAHOMA AVE`).
