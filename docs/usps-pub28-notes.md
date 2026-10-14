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
the live HTML pages supplied by the maintainer; the other tables are still
cross-checked only. Before release (Milestone 7), re-check every table
against the live Pub 28 pages and clear the items marked *pending* below.

| Table | Pub 28 source | Cross-check | Status |
|---|---|---|---|
| `suffixes.toml` | Appendix C1, Street Suffix Abbreviations (<https://pe.usps.com/text/pub28/pub28apc_002.htm>) | Copy of the C1 HTML table (October 2026). First transcribed from usaddress-scourgify `STREET_TYPE_ABBREVIATIONS` and street-address gem 2.0.0 `STREET_TYPES` (both MIT). | **Verified**: 202 abbreviations and every C1 spelling. The check found COMMONS/CMNS and the spelling CAUSWA missing (added), confirmed CROSSROADS/XRDS and TRAILER/TRLR, and found 20 spellings in the open-source copies that C1 does not list (CRT, CK, CR, LA, TR, PK, ...); those are now `observed`. C1 quirks kept as is: it lists MDW under both MEADOW and MEADOWS (we map MDW to MEADOW only; see the table comment), and it labels the ANX row's primary name ANEX (we use ANNEX). |
| `units.toml` | Appendix C2, Secondary Unit Designators (<https://pe.usps.com/text/pub28/pub28apc_003.htm>) | Copy of the C2 HTML table (October 2026). First transcribed from usaddress-scourgify `OCCUPANCY_TYPE_ABBREVIATIONS`. | **Verified**: all 24 designators and abbreviations, and the `range_required` column (C2 marks BSMT, FRNT, LBBY, LOWR, OFC, PH, REAR, SIDE, UPPR as not needing a secondary range). C2 spells the HNGR description "Hanger"; HANGAR is kept as `observed`. C2 also has a "Blank, unable to determine" row that requires the pound sign (#) on the mailpiece; that is the `#` designator the tagger handles. |
| `states.toml` | Appendix B, Two-Letter State and Possession Abbreviations (<https://pe.usps.com/text/pub28/28apb.htm>) | usaddress-scourgify `STATE_ABBREVIATIONS` and the street-address gem `STATE_CODES` for names and codes. | Cross-checked: the gem agrees on all 59 codes and names for states, DC and possessions. *Pending*: Armed Forces codes AA/AE/AP (in neither source). |
| `directionals.toml` | Chapter 2, directionals | usaddress-scourgify `DIRECTIONAL_REPLACEMENTS` and the street-address gem `DIRECTIONAL`. | Cross-checked. *Pending*: the Pub 28 section number. |
| `special.toml` PO Box, rural route, general delivery | Chapter 2, PO Box, rural route and general delivery addresses | None found. | *Pending*: the standard forms (PO BOX, RR, GENERAL DELIVERY) are well established, but the variant lists, the PO Box synonyms (DRAWER, CALLER, LOCKBOX) and the section numbers need checking against the text. |
| `special.toml` `[highway_contract]` | Section 251, Highway Contract Route Addresses: Format | Copy of the section 251 text and its example (October 2026). | **Verified**: the standard form is `HC ## BOX ##` (example: HC 68 BOX 23A). Section 251 names HIGHWAY CONTRACT and STAR ROUTE as words not to use, which confirms them as input-only variants. HCR, H C, HWY CONTRACT and STAR RTE are not named there; they stay as accepted input. |
| `special.toml` `[[highway]]` | Chapter 2, highway street names | None found. | *Pending*: which standard form Pub 28 uses for each highway prefix (for example COUNTY ROAD vs CR) must be confirmed before the USPS emitter (Milestone 3) relies on it. In Milestone 2 the table is only used to *recognize* highway names. |
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
  road; ST is street and saint). The lexicon answers each table on its own.
  Choosing between them by position is the tagger's job.
- State-dependent abbreviations (SR for State Road in Florida vs State Route
  elsewhere; RR for Ranch Road in Texas) are left out of the base tables, to
  be added by state packs.

## Emitter rules

TODO: written in Milestone 3. Rules already sourced:

- Highway contract route (section 251): emit `HC <route> BOX <box>`, e.g.
  `HC 68 BOX 23A`. Drop HIGHWAY CONTRACT, ROUTE, NUMBER, NO, STAR ROUTE and
  the pound sign, so `HIGHWAY CONTRACT ROUTE 68 BOX 23A`,
  `STAR ROUTE 68 BOX 23A` and `HC RTE #68 BOX 23A` all become
  `HC 68 BOX 23A`. The route and box values may hold letters (23A).
