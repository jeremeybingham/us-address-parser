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
each table was transcribed and then **cross-checked against independent
open-source transcriptions** rather than against the USPS page itself. Before
release (Milestone 7), re-check every table against the live Pub 28 pages and
clear the items marked *pending* below.

| Table | Pub 28 source | Cross-check | Status |
|---|---|---|---|
| `suffixes.toml` | Appendix C1, Street Suffix Abbreviations (<https://pe.usps.com/text/pub28/28apc_002.htm>) | usaddress-scourgify `STREET_TYPE_ABBREVIATIONS` and street-address gem 2.0.0 `STREET_TYPES` (both MIT). The two agree on every spelling-to-abbreviation pair; they differ only in whether some standard abbreviations are listed as mapping to themselves. The table is their union: 199 abbreviations. | Cross-checked. *Pending*: `CROSSROADS`/`XRDS` and `TRAILER`/`TRLR` are in neither source and were added from Appendix C1 from memory; confirm. |
| `units.toml` | Appendix C2, Secondary Unit Designators (<https://pe.usps.com/text/pub28/28apc_003.htm>) | usaddress-scourgify `OCCUPANCY_TYPE_ABBREVIATIONS`: agrees on all 24 designators, except that it misspells HANGAR as HANGER and gives UPPER instead of the Pub 28 abbreviation UPPR. | Cross-checked. *Pending*: the `range_required` column (no number needed after BSMT, FRNT, LBBY, LOWR, OFC, PH, REAR, SIDE, UPPR) is not in the cross-check source; confirm against C2. |
| `states.toml` | Appendix B, Two-Letter State and Possession Abbreviations (<https://pe.usps.com/text/pub28/28apb.htm>) | usaddress-scourgify `STATE_ABBREVIATIONS` and the street-address gem `STATE_CODES` for names and codes. | Cross-checked: the gem agrees on all 59 codes and names for states, DC and possessions. *Pending*: Armed Forces codes AA/AE/AP (in neither source). |
| `directionals.toml` | Chapter 2, directionals | usaddress-scourgify `DIRECTIONAL_REPLACEMENTS` and the street-address gem `DIRECTIONAL`. | Cross-checked. *Pending*: the Pub 28 section number. |
| `special.toml` PO Box, rural route, highway contract, general delivery | Chapter 2, PO Box, rural route, highway contract route and general delivery addresses | None found. | *Pending*: the standard forms (PO BOX, RR, HC, GENERAL DELIVERY) are well established, but the variant lists, the PO Box synonyms (DRAWER, CALLER, LOCKBOX) and the section numbers need checking against the text. |
| `special.toml` `[[highway]]` | Chapter 2, highway street names | None found. | *Pending*: which standard form Pub 28 uses for each highway prefix (for example COUNTY ROAD vs CR) must be confirmed before the USPS emitter (Milestone 3) relies on it. In Milestone 2 the table is only used to *recognize* highway names. |
| `special.toml` `[placeholder]` | Not a USPS rule | auction-tracker `PLACEHOLDER_RE` and its tests (see `docs/prior-art.md`). | Data-quality list; no USPS claim. |

### Non-Pub 28 spellings

Spellings seen in real data that Pub 28 does not list are kept apart from
the Pub 28 forms, in `observed = [...]` lists with a source comment. Lookups
return them with `MatchKind::Observed` so the parser can report
`Warning::NonStandardAbbreviation`. Current entries, all from auction-tracker
(Florida county data):

| Spelling | Resolves to | Table |
|---|---|---|
| `TRC` | TRCE (TRACE) | suffixes |
| `UN` | UNIT | units |
| `LT` | LOT | units |

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

TODO: written in Milestone 3.
