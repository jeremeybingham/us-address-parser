# USPS Pub 28 notes

Sources for the Pub 28 tables and rules this library implements. Every USPS
rule in code must trace to an entry here (see `CLAUDE.md`).

Publication: USPS Publication 28, *Postal Addressing Standards*,
<https://pe.usps.com/text/pub28/welcome.htm>. Table edition targeted: the
current online edition as of October 2026.

Formatting to Pub 28 is not validation. Nothing here checks that an address
exists or is deliverable.

## Lexicon tables (Milestone 1)

`data/*.toml` is compiled into static tables by `crates/doorplate/build.rs`
and read only through `Lexicons` (`crates/doorplate/src/lexicon.rs`).
`build.rs` rejects malformed tables: a spelling that maps to two entries, a
duplicate abbreviation or code, a form that is not a normalized key, or an
entry whose `forms` leave out its own name or abbreviation.

### Verification status

Each table was first transcribed from independent open-source copies
(usaddress-scourgify and the street-address gem, both MIT), then checked in
October 2026 against the live Pub 28 pages: Appendix C1 and C2 from a copy
of the HTML supplied by the maintainer, Appendix B, Appendix F and the
chapter 2 sections cited below fetched directly (`pe.usps.com` sends
clients without a browser User-Agent to a "page not found" page). Before
release, re-check every table against the live pages (`docs/design.md`,
item 44, Milestone 8).

**Re-checked 2026-10-17 (item 44).** Every table in the list below was
compared with the live pages by `tools/pub28/verify.py` (it parses Appendix
C1, C2 and B and diffs them against `data/suffixes.toml`, `units.toml`,
`states.toml` and `directionals.toml`; `--emitter` also runs every Appendix F
example through the parser and the USPS emitter): **0 differences** in 202
suffix rows and their spellings, 24 designators and their range flags, 62
state and military codes, 8 directionals. The rule sections cited here and in
the emitter section (211 to 215, 221 to 224, 225 to 239, 241 to 246, 251 to
255, 261, 262, 281 to 285, 341, 343, 35, 354, 356, Appendices A1, D1, D2, D4
and F) were read again and say what this file says. The 238.1 and 239 examples
are images and were not re-read. The check fetched the pages with a browser
User-Agent. Two Appendix F examples failed and are fixed (see "Added
2026-10-17" below); a third, `SR MM`, is left to state packs on purpose. One
typo on the USPS side: the C1 page source has a literal `\u2009BYU` where
BAYOU's abbreviation should be `BYU`; the tool strips it.

Chapter 2 page map (all under `https://pe.usps.com/text/pub28/`): 225
Military Addresses `28c2_010.htm`; 233 Directionals `28c2_014.htm`; 237
Highways `28c2_018.htm`; 24 Rural Route `28c2_021.htm` to `28c2_026.htm`
(241 to 246); 25 Highway Contract Route `28c2_027.htm` to `28c2_031.htm`;
26 General Delivery `28c2_032.htm` and `28c2_033.htm`; 28 Post Office Box
`28c2_036.htm` to `28c2_040.htm` (281 to 285). Appendix F is `28apf.htm`.

| Table | Pub 28 source | Cross-check | Status |
|---|---|---|---|
| `suffixes.toml` | Appendix C1, Street Suffix Abbreviations (<https://pe.usps.com/text/pub28/pub28apc_002.htm>) | Copy of the C1 HTML table (October 2026). First transcribed from usaddress-scourgify `STREET_TYPE_ABBREVIATIONS` and street-address gem 2.0.0 `STREET_TYPES` (both MIT). | **Verified**: 202 abbreviations and every C1 spelling. The check found COMMONS/CMNS and the spelling CAUSWA missing (added), confirmed CROSSROADS/XRDS and TRAILER/TRLR, and found 20 spellings in the open-source copies that C1 does not list (CRT, CK, CR, LA, TR, PK, ...); those are now `observed`. C1 quirks kept as is: it lists MDW under both MEADOW and MEADOWS (we map MDW to MEADOW only; see the table comment), and it labels the ANX row's primary name ANEX (we use ANNEX). |
| `units.toml` | Appendix C2, Secondary Unit Designators (<https://pe.usps.com/text/pub28/pub28apc_003.htm>) | Copy of the C2 HTML table (October 2026). First transcribed from usaddress-scourgify `OCCUPANCY_TYPE_ABBREVIATIONS`. | **Verified**: all 24 designators and abbreviations, and the `range_required` column (C2 marks BSMT, FRNT, LBBY, LOWR, OFC, PH, REAR, SIDE, UPPR as not needing a secondary range). C2 spells the HNGR description "Hanger"; HANGAR is kept as `observed`. C2 also has a "Blank, unable to determine" row that requires the pound sign (#) on the mailpiece; that is the `#` designator the tagger handles. Added outside C2: PMB (PRIVATE MAILBOX), from section 285, which names PMB or `#` before a private mailbox number (<https://pe.usps.com/text/pub28/28c2_040.htm>). |
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
| `LP` | LOOP | suffixes |
| `UN` | UNIT | units |
| `LT` | LOT | units |

Seen in DBPR licensee data (Florida real estate licensees, 2025 export;
counts are rows out of about 445,000 Florida addresses):

| Spelling | Resolves to | Table |
|---|---|---|
| `BLV` (96), `BL` (46) | BLVD (BOULEVARD) | suffixes |
| `TE` (50, Miami-Dade grid streets: `NW 127 TE`) | TER (TERRACE) | suffixes |
| `APTO` (170), `AP` (38), `APART` (37), `APARTAMENTO` (20), `APPT` (20), `APARTAMENT` (13), `APP` (13), `APRT` (10) | APT (APARTMENT) | units |
| `SUIT` (35) | STE (SUITE) | units |

`AP` is also the Armed Forces Pacific state code; the tagger reads the
last line first, so `APO AP 96xxx` keeps it as the state.

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
  be added by state packs. A pack can also read a base spelling differently:
  the Florida pack reads the observed CREEK spelling `CR` as CIRCLE
  (`data/packs/fl/overrides.toml`; evidence in `docs/packs/fl.md`).

## Parser rules (Milestone 2)

How the tagger (`crates/doorplate/src/tagger/`) reads an address, and where
each rule comes from. Rules from Pub 28 cite its section; the rest come from
shapes in real county data (`docs/prior-art.md`) and are not USPS rules.

| Rule | Source |
|---|---|
| Read the last line (ZIP, state, city) first, then the delivery line from both ends: units and postdirectional from the right, house number and predirectional from the left. | Section 231 (parse from the right-most element). |
| Directionals: one or two words before the name are the predirectional, after the name and suffix the postdirectional, abbreviated to the Appendix B forms. A directional with no name left beside it is the name (`100 N ST`, `2 SOUTH ST`). The four two-letter forms written as two letter-words (`N W`, `N E`, `S W`, `S E`) are listed spellings in `directionals.toml`. | Sections 233.1, 233.21, 233.22; Appendix B; the spaced spellings from prior art (DBPR data, `8491 N W 17TH STREET`). |
| A one-letter directional right after a lone suffix word is part of the name, like any letter there (`AVENUE N`, `AVENUE G`). | Section 233.3 (letters used as alphabet indicators); prior art (`2108 AVENUE G`). |
| Suffix: the last word before units and postdirectional, when a name is left before it. Suffix words earlier in the line stay in the name (`EAGLES POINT CIRCLE`). | Section 234; Appendix C1. |
| Highway names: a prefix from `[[highway]]` right after the house number (or a predirectional) and followed by a route that starts with a digit. The prefix and route go in `highway`; later words are suffix, postdirectional or name (`HWY 11 BYPASS`, `US 41 SW`). `I10` and `IH280` are split by the tokenizer. | Section 237; Appendix F. |
| A state code or full name (not a traditional abbreviation or military code) right before a highway prefix goes in `highway.state` (`CA COUNTY RD 150`, `KY ST HWY 1`). A state right before an all-digit route, with no prefix, gets the HIGHWAY prefix (`KENTUCKY 440`, `KY 1207`). Tried after the plain and predirectional forms, so `NE HWY 5` keeps NE as a directional; the bare form skips states that are also directionals (`NE 5TH ST`). A state that is also a designator counts (`5100 FL 40 SUITE 100` is `FL HIGHWAY 40`): at the start of the street, a floor would leave no street name. | Appendix F and its note (a state that is the whole primary name, `OKLAHOMA AVE`, is the name). |
| County grid numbers are house numbers: digits and one or two compass letters, each followed by digits (`22W410`, `5S710` in DuPage County, IL; `W338S9739`, `S1749` in Wisconsin). | Appendix D2 (grid style addresses; it names the Salt Lake City form, not these); prior art (DBPR out-of-state data). |
| Two plain house numbers of three or more digits (`1309 1329 E VINE ST`, `4417 4417 N OAK AVE`): the first is the address's, the rest are left unparsed with `multiple_addresses`, only when a whole street follows on the line (an optional directional, a name word, the suffix ending the line). Rural grid streets named by number and direction (`1575 NORTH AVE`, `275 NORTH RD E`) and a split ordinal (`139 TH AVE`) keep the number in the name. | Prior art (the hand-labeled Florida sample, DBPR data); Appendix D2 (grid names). |
| `P O BOX 4417 N E 9TH AVENUE`: a box word and number followed by a directional, an ordinal or number name and a suffix on the same line is a street address typed with stray box words; the box words go to `unparsed`. A named street (`PO BOX 12 N MAIN ST`) stays a box. | Prior art (the hand-labeled Florida sample, DBPR data); no USPS rule. |
| House number `N-N`: one hyphenated number (`112-10`, `289-01`), unless the second part has as many digits and is larger (`1204-1206`), which raises `multiple_addresses`. | Appendix D1, section 232; the range from prior art. |
| Units: a designator and value (`APT 4B`), `#` and value, a designator with `#` (`UNIT # 4`), or a designator alone (`REAR`, or `UNIT` with a missing number when Appendix C2 requires a range). Up to two, the first in input order going to `unit_*`. An ordinal before a designator is its value (`3RD FLOOR`, kept as given). PMB is a designator (`STE 201 PMB 308`). | Section 213; Appendix C2 (designators, range column, `#` when the designator is unknown); section 285 (PMB); the ordinal form from prior art (DBPR data). |
| A postdirectional alone after a comma belongs to the street (`2272 GOODWOOD BLVD., SE`). | Section 233; prior art (DBPR data, Atlanta and DC addresses). |
| A later line or comma segment that is only a number is a unit with no designator (`555 SOUTH STREET` / `4001`), when a street name of two words or more comes first (`1921 NW` / `46` is a street cut across lines). | Section 213 (`#` when the designator is unknown); prior art (DBPR data). |
| Attention line: from an `[attention]` word (ATTN, ATTENTION, C/O, CARE OF) to the end of its line or comma segment, at the start or end of the address lines, goes in `attention` (`323 NORRISTOWN ROAD` / `ATTN LAW DEPT`, `PO BOX 23181 C/O YMF`). A C/O right before a house number drops only the word (`C/O 2775 NW 49TH AVE`). | Section 214; C/O from prior art (DBPR data). |
| Dual address: a PO Box and a street with a house number in one record, in either order, on one line or two (`9418 CUBA RD` / `P.O. BOX 100`, `PO BOX 286` / `23320 NE HWY 315`, `... AVE PO BOX 365`). Both are kept, with `dual_address`. A bare `BOX 88` counts when it starts a later line or segment. A second line with no house number is not a street (`PO BOX 3163` / `GRAND CENTRAL STATION`). | Section 215; shapes from prior art (DBPR data). |
| A delivery line written twice is read once (`PO BOX 12442 PO BOX 12442`, `42 S MAIN ST 42 S MAIN ST`). | Prior art (DBPR data). |
| Bare units: after a suffix (and optional postdirectional), a value with a digit or a single letter, or a letter and a number (`DR 706`, `DR B`, `DR V 21`). Not a directional letter (`PARK AVE S` is a postdirectional), not after a highway word other than ROAD, and only when a name comes before the suffix. | Prior art (Florida county data). |
| PO Box: a `[po_box]` form or synonym, then an optional `#` and the box value. | Sections 281, 283. |
| Rural route and highway contract route: the route word, optional `#`, route number, then `BOX` and the box value. RD, RT, RTE and ROUTE (also highway prefixes or suffixes) need the BOX part. | Sections 241, 244, 251, 253. |
| General delivery: the `[general_delivery]` forms. | Section 261. |
| Military: a `[[military]]` type, its number, then `BOX` and the box value; APO, FPO or DPO before AA, AE or AP takes the city's place. | Sections 225.1, 238.1, 239. |
| ZIP: the last token, 5 or 9 digits or `5-4`, not after `#`. A shorter all-digit token after a state is an invalid ZIP when the state starts its line or the input is at least 30 characters long. | ZIP formats; truncation from prior art. |
| State: right before the ZIP (traditional abbreviations such as FLA allowed), else first in the ZIP's line. With no ZIP, only a USPS code or full name ending a later line, or ending a one-line address after a city. On one line with no commas, a state that is also a street suffix, with no city before it, is the suffix (`12 SIMON CT 32505` is COURT, not Connecticut). In multi-field input, a state ending the address line that repeats the state field is dropped (`4486 W OAKLAWN ST LECANTO FL`). | Appendix B; prior art (`FL- PORT RICHEY 34654`, Escambia and DBPR data). |
| City: the line before the state or ZIP line, or the words before the state on its line. On a line with no commas, the words after the street's suffix (and any postdirectional and unit). A one-line address whose street has suffix words in the city (`MOUNTAIN VIEW`) needs a place list, which is a state-pack job. | Prior art; design section 11. |
| A copy of the city at the end of the street line, possibly cut off, is dropped (`... DR FERNANDINA B`). Never dropped this way: a suffix; a value right after a designator or `#` (`SUITE A` before `ARCHER`); a single cut-off letter (`AVE S` before `ST PETERSBURG`, `SAXONY D` before `DELRAY BEACH`). | Prior art (Nassau; the exceptions from Pinellas, Palm Beach and DBPR data). |
| Multi-field input: a line 2 that repeats the end of line 1 is a copy and is skipped (`9453 LUTOO LANE` / `LUTOO LANE`). | Prior art (DBPR data). |
| Placeholders: a whole street line in `[placeholder]`, or a placeholder before ` - ` (dropped, or the whole line when only a code follows). Asterisks around words are trimmed (`**Private Address**`); a line of only punctuation, asterisks included, is a placeholder. | Prior art; not a USPS rule (section 354 lists asterisks for removal). |
| Trailing country: `USA`, `US`, `UNITED STATES`, `UNITED STATES OF AMERICA` after a comma, a ZIP or a state (or as a whole field other than the street line) is cut before tagging and raises `country_ignored` (decision 36; `data/special.toml` `[country]`). `US` in a highway name or after a street is kept. Other countries are not recognized. | Not a USPS rule: Pub 28 addresses are domestic and have no country line. Seen in real data (`co-001` to `co-015`). |
| Intersections: `&`, `@`, `AND` or `AT` in one comma segment, after a name and a street suffix (a directional may follow, and a house number or directional may lead) and before words that run to a street suffix, split the input. The first street is parsed, the connector and second street go to `unparsed`, `intersection_detected` is raised, and `key()` and `slug()` are `None`. A suffix word alone is a name (`BEACH AT SEA DR`); a suffix directly before another is part of the name (`& PARK AVE`). `LAND AND SEA RD`, `STATE AND MAIN ST`, `AT&T WAY` are single streets. | Not a USPS rule: Pub 28 does not address intersections. Decision 37; corpus `is-001` to `is-013`. |
| A street suffix glued to a city (`STREETDADE CITY FL`) is split only when the city is anchored for real: a state or ZIP in the same segment ends it only if nothing but a ZIP follows and no later segment holds a state or ZIP; the end of the segment ends it only if the next segment starts with a state or ZIP and the city's last word is not a street suffix; a word that completes a place with the words before it (`PICO RIVERA`) is never split. Before this, any word that began with a suffix spelling and ended in a place name was split (`RIVERA` into `RIV ERA`, `CREEKWOOD`, `AVENIDA DE LA REINA`, `STRAND`). | Not a USPS rule. Found by the Netsyms comparison (docs/future-sources.md); corpus `ns-001` to `ns-008`. |
| A word after `#` is the unit value (`# PRIMARY`, `# REAR`), and a letter range is a unit value (`APT A-D`). | Not a USPS rule; Pub 28 section 213 writes `#` with any secondary number. Seen in real data (corpus `ns-009`, `ns-010`, `ns-014`). |
| A value right after `#` or a unit designator is not split by the glue rules: `# 2WD` is not house number 2 and a name, `APT I4` is not Interstate 4. | Not a USPS rule. Corpus `ns-011`, `ns-012`. |
| A single letter after a house number is not a number suffix when it is `U` before `S` (`20192 U S ROUTE 23`). | Not a USPS rule; `U S` is the written form of US. Corpus `ns-013`. |
| A postdirectional is not a cut-off copy of the city: `1040 HANCOCK CHAPEL RD NE, NEW SALISBURY, IN` keeps NE. | Not a USPS rule. Corpus `ns-015`, `ns-016`. |
| On one line with no commas, a state code that is also a suffix spelling (CT, LA, WY, KY, MT, PR) is the state, not the suffix, when the ZIP belongs to that state and a place of that state stands right before it (`9 ELM WALLINGFORD CT 06492`); with a ZIP of another state or no such place it stays the suffix (`12 SIMON CT 32505`). `WASHINGTON DC` before a ZIP is the city and DC, though the whole phrase is also a spelling of the state. | Not a USPS rule. The ZIP-to-state and place data are `data/places/` (Census Gazetteer, GeoNames; docs/places.md). Corpus `ns-017` to `ns-023`. |
| Truncation: `possibly_truncated` when a line of at least 30 characters ends in something incomplete: a cut-off ZIP, a designator without its number, a cut-off designator (`UNI`; a street suffix such as `ST` or `TRL` is never one, though it starts STE or TRAILER), or a name with no suffix. | Prior art (county sites cut lines at about 30 characters). |
| The parser does not guess: `409742ND` (4097 42ND or 40974 2ND), slash pairs (`5210/5212`), comma lists of house numbers (`31967,32037,...`, `multiple_addresses`) and anything no rule claims go to `unparsed`. | Prior art. |

Added 2026-10-15 from the usaddress comparison and the geocode benchmark
(`docs/usaddress-comparison.md`; corpus cases `rg-050` to `rg-081`). Rules
marked "decided" were chosen by the author:

| Rule | Source |
|---|---|
| One line with no commas: a highway route ends the street, then an optional postdirectional, a suffix after a name word (`HWY 66 FRONTAGE RD`) and a unit; the city follows (`395 COUNTY ROAD 8 LAUREL MS`). A directional with nothing after it is the postdirectional (`HIGHWAY 29 SOUTH 32533`). | Section 237; prior art (geocode benchmark). |
| Name words after a highway route belong to the street only on the route's line or comma segment; the emitters and the key keep them (`HIGHWAY 66 FRONTAGE RD`). Words on a later line are unparsed. | Appendix F; bug fix (the word was dropped from every output). |
| A street and city in the first comma segment are split after the street (`626 E KILBOURN AVE MILWAUKEE, WI`), unless the segment ends in a suffix or directional (`427 SOUTH LA SALLE STREET, 60605` is all street). | Prior art (usaddress test data). |
| A unit starting the city's segment belongs to the street (`..., # 234 SANTA FE NM`). | Bug fix (usaddress test data). |
| A unit before the street is the first unit (`APT 1B 626 E KILBOURN AVE`); a unit after the street is then the second. | Bug fix (usaddress test data). |
| No ZIP, one line: a USPS state code or name ending the street is the state (`8517 CEDAR BARK CIR AL`). Never NE or FL (a directional and a floor). CT, KY, LA, MT, PR, WY and FM (also suffix or highway words) only right after a suffix written as an abbreviation shorter than the word (`CIR KY`; not `HIDDEN MEADOW CT` or `OAK PARK CT`). | Appendix B; decided. |
| Hyphenated routes are split: `GA-25`, `US-385`, `CR-48`, `I-65`, when the letters are a highway prefix or a state code that is not a directional. `SR-54` stays whole (SR is left to state packs); `NE-2` stays whole. | Appendix F (written with a space); decided. |
| A delivery line that is only `BOX` and a number is a PO Box (`BOX # 123456`). | Section 283 names CALLER, BIN, LOCKBOX and DRAWER, not plain BOX; decided. |
| Military box line in reverse order: `BOX 2877 PSC 103` is `PSC 103 BOX 2877`. | Section 238.1. |
| A single letter between the house number and a directional belongs to the number (`428 B SOUTH MAIN ST`). Before a name word it is an initial in the name (`820 A PHILIP RANDOLPH BLVD`, `9058 T BIRD RD`). | Decided, narrowed after real data. |
| A letter followed by two digits or more is a house number at the start of a street (`T703 STATE ROUTE 66`); a letter alone is not. | Decided. |
| A county grid number written in two parts is one house number, kept as written (`N165 W2123`). | Decided. |
| After COUNTY ROAD or COUNTY HIGHWAY, a route of one or two letters (`COUNTY ROAD J`). | Appendix F (letter routes); decided. |
| `COUNTRY ROAD` / `COUNTRY RD` before a route number is an observed typo for COUNTY ROAD (`non_standard_abbreviation`); without a number it stays a street name. `HGWY` is an observed spelling of HIGHWAY (highway prefix and HWY suffix). | Prior art (usaddress test data, DBPR data); decided. |
| HC with ROUTE written out: `HC ROUTE`, `HC RTE`, `HC R`, `HWY CONTRACT ROUTE`, `HIGHWAY CONTRACT RTE`. | Section 251 (drop ROUTE). |
| A stray backtick is trimmed (`1719\``). | Section 354 (punctuation removal). |
| A house number and a unit with no street raise `missing_street_name` (`123 UNIT 1`). | Decided. |
| Floors are numbers: an ordinal floor value is written without its ordinal suffix (`5TH FLOOR` is FL 5). | Appendix C2 (FL takes a range); decided. |
| Building name: words ending in a `[building]` word (TOWER, BUILDING, HALL, CENTER, PLAZA, ...), at the end of the delivery lines, in their own line or comma segment or right after a unit (`SUITE 801 EAST TOWER`, line 2 `ANNEX BUILDING`), go in `building`. Not after the house number alone (`1 TRUMP TOWER` is a street). | Not a USPS rule (Pub 28 has no building line); usaddress BuildingName; decided. |

Added 2026-10-16 from the DBPR licensee data (Florida; corpus cases
`rg-082` to `rg-097`):

| Rule | Source |
|---|---|
| A line 2 or comma segment that is exactly the city field is a copy and is skipped, even when its first word is also a suffix (`CAPE CORAL`, `FORT MYERS`, `LAKE WORTH`). The cut-off-copy rule above still keeps a suffix at the end of the street line. | Prior art (DBPR data). |
| A unit value written in two parts, a letter and a number in either order, right after a designator or `#` and in the same line or comma segment, is one value kept as written (`APT E 1003`, `UNIT 210 C`, `# 104 M`), like the bare `DR V 21`. Two numbers stay unparsed (`STE 109 258`). | Section 213; prior art (DBPR data). |
| A street cut across lines: when line 1's street part has no suffix and the next line or comma segment is a suffix, with at most a number before it and a directional after it, the two are one street (`8725 NW` / `18 TER`, `6556 CHESTNUT` / `CIR`, `750 NE` / `64 ST APT B202`). A line 2 of name words stays unparsed (`... OAK COURT` / `MAGNOLIA TERRACE`). | Prior art (DBPR data). |
| A dash between the street and its unit is dropped (`3RD AVENUE - SUITE 1500`, `192ND STREET - #502`). | Section 354 (punctuation removal); prior art (DBPR data). |

Added 2026-10-16 from the place list, the Florida pack and three more DBPR
licensee files (corpus `fl-020` to `fl-033`, `rg-098` to `rg-120`, `ds-001`
to `ds-007`). Most are data-quality rules with no USPS text behind them; the
source column says what does back each one:

| Rule | Source |
|---|---|
| A state pack may add a highway prefix spelling the base table lacks, read in that state only: `SR` is STATE ROAD in Florida (`22200 SR 60`), `CTY HWY` is COUNTY HIGHWAY (observed). Elsewhere `SR` stays a street name. | Appendix F maps `SR` to both STATE ROAD and STATE ROUTE, which is why it is state-dependent; Florida choice from DBPR evidence (139 lines, none STATE ROUTE); `docs/packs/fl.md`. |
| `DRT` is `DR`, `LNE` is `LANE`, `SU` is `SUITE` (observed spellings, `non_standard_abbreviation`). | Not in C1 or C2; DBPR licensee data (`docs/packs/fl.md`). |
| A penthouse right after a designator is that designator's value: `APT PH 5213` is `APT PH5213`, `SUITE PH` is `STE PH`. | Appendix C2 lists PH as a designator that takes no range; joining is decided (design.md decision 31); DBPR (162 lines). |
| A designator joined to its value by `.`, `:` or `-`, or run into a value that starts with a digit (`APT.1285`, `STE.200`, `APT:201`, `UNIT-4`, `APT3D`), is split. | Section 354 (punctuation is removed); Appendix C2 designators; DBPR (about 1,200 lines). |
| `BAY`, `STUDIO` and `BOOTH` followed by a value are a unit, written `UNIT` (observed spellings). A bare observed spelling with no value (`GULF TO BAY`) is not a unit. | C2 has no such words; its "blank, unable to determine" row takes `#`, and UNIT is the nearest listed designator; DBPR (127, 99 and 11 lines); decided. |
| `NO`, `NUM` or `NUMBER` between a designator and its value is dropped (`SUITE NO. 102` is `STE 102`). | Section 241 tells writers not to use NO or NUMBER in a rural route address; extended to unit values; decided; DBPR. |
| A cut-off copy of the address ahead of the whole one (`624 E COLONI 624 E COLONIAL DR`): the house number comes again after words that match the start of what follows, in one segment; the copy is dropped. | Data-quality rule, no USPS text; DBPR mailing fields (about 1,785 lines). |
| A name or building on line 1 (or in the first comma segment) with a house-numbered street on the next is the attention line. | Section 214 (an attention line goes above the address); DBPR (629 lines). |
| `default_state`: when the input names no state and has no known ZIP, a city is looked up in the hinted state and that state's pack readings (`CR`, `SR`) apply. It never fills `state`. | Library behavior, not a USPS rule (design.md decision 24). |
| After a highway prefix, a route that starts with a letter: one letter, digits, at most one more letter (`HIGHWAY A1A`, `STATE ROAD A1A`). Elsewhere the route still starts with a digit. | Appendix F (routes after a prefix); Florida State Road A1A observed in Florida address data (407 rows were street names); corpus `rg-121`. |
| After a highway route, the route anchors the units the way a street name does: a later line that is only a number is a bare unit (`HWY 27` / `405`). | Not a USPS rule; Florida address data, found while labeling the blind Florida sample (design.md item 36); corpus `rg-122`, `rg-123`. |
| A number of one to three digits run into `ST` or `RD` where that is not the number's ordinal ending is the number plus the suffix: `178ST` is `178 ST` (178's ordinal is `178TH`); `21ST` and `3RD` stay whole; four digits or more stay whole (`is_glued_ordinal`). A cut-off copy ahead of the address is compared without word breaks, so it is still found after the split. | Not a USPS rule; Florida address data, found while labeling the blind Florida sample (design.md item 36) (about 1,050 rows such as `NE 199ST`); corpus `rg-124` to `rg-126`. |
| A directional letter right after the suffix on its line, then a number, is the postdirectional and a bare unit (`DR E 203` is `DR E`, unit 203); the letter-and-number unit (`DR V 21`) stays for other letters and for a line of its own (`E 203`). | Section 233 (postdirectional after the suffix); Not a USPS rule; Florida address data, found while labeling the blind Florida sample (design.md item 36); corpus `rg-127`, `rg-128`. |
| `UNIT # 4` takes the designator before `#` only when no ordinal after the street's suffix comes first: `RD 2ND FLOOR #260` is floor 2 and unit 260 (`SW 11TH APT #2` keeps 11TH as the name). | Appendix C2 (FL takes a range); Not a USPS rule; Florida address data, found while labeling the blind Florida sample (design.md item 36); corpus `rg-129`, `rg-130`. |
| `PH FLOOR` is one unit, the penthouse: `PH`, and the floor word is dropped. | Appendix C2 (PH needs no range); Not a USPS rule; Florida address data, found while labeling the blind Florida sample (design.md item 36) (author's call); corpus `rg-131`. |
| After a whole unit (designator or `#`, then a value), an exact copy of the city is dropped even when it starts with a suffix word (`UNIT 412 INLET BEACH` before `INLET BEACH`). As a last resort, a whole comma segment that repeats the city's last words is a cut-off copy (`RD,CLOUD` before `ST CLOUD`); a copy across the comma (`RD WESLEY` / `CHAPEL`) is still dropped whole first. | Not a USPS rule; Florida address data, found while labeling the blind Florida sample (design.md item 36); corpus `rg-132` to `rg-134`. |
| A period after an abbreviation of two letters or more, with a letter right after it, separates words (`ST.CLOUD` is `ST CLOUD`, `CIR.APART`, `BLVD.N`), unless the pieces join into a known word (`BLV.D` stays `BLVD`); initials stay joined (`P.O.`, `U.S.A.`). A box word joined to its number by punctuation is split like a designator (`BOX.2351`). | Not a USPS rule (Pub 28 drops periods); Not a USPS rule; Florida address data, found while labeling the blind Florida sample (design.md item 36); corpus `rg-133`, `rg-135`, `rg-136`. |

Added 2026-10-17 from the Pub 28 re-check (item 44; corpus `p28-twosuf-001`
to `006`, `p28-highway-038` to `040`):

| Rule | Source |
|---|---|
| Two suffix words in a row: the second is the suffix, and the first is part of the street name, written out in full (`HARBOR VW DR` is `HARBOR VIEW DR`, `GULL PT RD` is `GULL POINT RD`, `BISCAYNE BLVD WAY` is `BISCAYNE BOULEVARD WAY`). Only a Pub 28 standard abbreviation is spelled out; a spelling C1 lists but is not the abbreviation, or an observed one (`PK`), stays as written. After a number, `ST` and `RD` are the ordinal's ending and stay (`63 RD TER`, `21 ST AVE`). The name and the key change with it. | Section 234.2; Appendix F (`HIGHWAY 3 BYP ROAD` is `HIGHWAY 3 BYPASS RD`). |
| A highway route after a prefix keeps up to two letters run into its number (`ROUTE 1150EE`, `COUNTY ROAD 60EE`); before, the letters were split off as a street name. Elsewhere a longer word run into a house number is still split (`101BRADDOCK`). | Appendix F (`ROUTE 1150EE`, `COUNTY HIGHWAY 60E`). |

## Emitter rules

### USPS emitter (`emit/usps.rs`, `ParsedAddress::to_usps`)

Output: `UspsAddress { line1, line2, city, state, zip }`, plus `last_line()`
(`CITY ST ZIP`) and `lines()` (the lines in mailpiece order). The layout
rules below were checked against the live Pub 28 pages in October 2026
(chapter 2 sections 21x to 23x, chapter 3 sections 34x and 35x, Appendices
A1, D and F). Page map: 211 `28c2_001.htm`; 212 `28c2_002.htm`; 213
`28c2_003.htm`; 221 `28c2_006.htm`; 222 `28c2_007.htm`; 223 `28c2_008.htm`;
224 `28c2_009.htm`; 231 `28c2_012.htm`; 232 `28c2_013.htm`; 238
`28c2_019.htm`; 239 `28c2_020.htm`; 341 `28c3_012.htm`; 343 `28c3_014.htm`;
35 `28c3_015.htm`; 354 `28c3_019.htm`; 356 `28c3_021.htm`; A1
`28apa_002.htm`; D1 `28apd_002.htm`; D4 `28apd_005.htm`. The examples in 238
and 239 are images (`images/28c2_019_1.jpg`, `28c2_019_2.jpg`,
`28c2_020_1.jpg`).

| Rule | Source |
|---|---|
| Uppercase. | Section 212 (uppercase preferred); A1. |
| Punctuation dropped (`. , ' " ; : ( )`), apostrophes included (`O'BRIEN` is `OBRIEN`). Kept: the hyphen in ZIP+4, house numbers and names (`289-01 MID-ISLAND PLZ`, `112-10 BRONX RD`), the slash in fractions (`123 1/2 MAIN ST`) and a period between digits (`39.2 RD`). `&` and `@` are left as given. | Section 222 (punctuation may be omitted, except the ZIP+4 hyphen); A1 (also keeps the hyphen in the primary or secondary number); section 232 (periods, slashes and hyphens, with these examples; hyphens in names may be replaced with a space, which we do not do); section 354 (the removal list, apostrophes included; `&` and `@` are on it only as an optional compression step); Appendix D1, D4. |
| Street line order: number, predirectional, name, suffix abbreviation, postdirectional, secondary unit, one space between. Directionals abbreviated. | Section 231; sections 233.1, 234 (as in the parser table above). |
| Secondary unit at the end of the delivery line (`APT 4`). A unit with no known designator is written `# 4`, with a space; a known designator drops the `#` (`APT #4` is `APT 4`). A designator with no number stands alone (`REAR`). Two units are written in input order. | Section 213 (preferred location; no `#` when the designator is known); 213.2 (space after `#`); Appendix C2. |
| No fixed line length. `to_usps()` keeps the unit on `line1`. `to_usps_wrapped(max_line)` moves the unit(s) to `line2` when `line1` would be longer than `max_line`. On a mailpiece `line2` goes immediately *above* `line1`; `lines()` returns that order. The field name follows the data-field convention (address line 2 holds the unit, as in `Fields`). | Section 35 (the database or label sets the line length); 213.3, 343.1 and 356 (a unit that does not fit goes on the line immediately above the delivery line). |
| Last line: `CITY ST ZIP`, one space between, no comma, ZIP+4 with a hyphen. Pub 28 prefers two spaces between state and ZIP but allows one; we use one, as its own examples do. | Section 224; 222 (comma omitted, hyphen kept); examples in 238, 239 and 341. |
| City names are written as parsed, spelled out or not. No SAINT/ST, FORT/FT or MOUNT/MT rule is applied: Pub 28 says to spell city names in full, and allows only the City State file's 13-character abbreviations, which this library does not have. | Sections 221, 223. |
| Military: a post office (APO, FPO, DPO) takes the city's place (`APO AE 09001`); the delivery line is `<type> <number> BOX <box>`, with box leading zeros kept (`UNIT 9900 BOX 0500`). | Sections 225.1, 238.1, 239 and their examples. |
| General delivery: `GENERAL DELIVERY`, spelled out, on the delivery line. | Section 261. |
| Rural and highway contract route numbers are written as given. 242 and 252 say a leading zero "is not necessary", not that it must go, and the key keeps the value as given (`RR 02` and `RR 2` have different `k1` keys), so the emitter does too. Box hyphens are kept as given (243, 252.1: only the ZIP+4 file can say). | Sections 242, 243, 252, 252.1. |
| A state name inside a highway name is written as its code (`KENTUCKY STATE HIGHWAY 625` is `KY STATE HIGHWAY 625`; `KENTUCKY 440` is `KY HIGHWAY 440`). A state name that is the whole primary name stays spelled out (`OKLAHOMA AVE`). | Appendix F and its note; section 358. See "Highways" below. |
| Dual address (street and PO Box): the PO Box alone, without the street's units. Pub 28 says to eliminate dual addresses on the mailpiece if possible, and that the delivery address is normally the PO Box. The key and the RESO output use the street. | Section 215 and its example (`28c2_005.htm`, image `28c2_005_1.jpg`). |
| The attention line is not emitted; Pub 28 places it above the recipient, outside the address lines this library writes. | Section 214. |
| Parts the parse did not find are left out. `diagnostics` is not consulted; unparsed text is not emitted. | Library behavior, not a USPS rule. |

### Pub 28 verification of the emitter (October 2026)

`pe.usps.com` was blocked when the USPS emitter was written, so its layout
rules were listed here to check against the live text. They were checked in
October 2026. What each check found:

1. **Punctuation.** Confirmed with two changes. Pub 28 lets punctuation be
   omitted except the ZIP+4 hyphen (222, A1). It keeps the hyphen in
   numbers and names, the slash in fractions, and a period in numbers such
   as `39.2 RD` (232). The emitter used to drop that period (`392 RD`); it
   now keeps it (corpus `p28-punct-001`). Apostrophes are on the 354
   removal list, so `O'BRIEN` is `OBRIEN`. A second finding: hyphenated New
   York style numbers (`112-10`, `289-01`, Appendix D1) raised
   `multiple_addresses`; only a range with a larger second number of the
   same length (`1204-1206`) does now.
2. **Secondary unit placement.** The unit goes at the end of the delivery
   line (213). When it does not fit, it goes on the line immediately above
   (213.3, 343.1, 356). Pub 28 sets no line length (35), so the 40-character
   placeholder is gone: `to_usps()` never wraps, `to_usps_wrapped(max_line)`
   wraps at the caller's limit, and `UspsAddress::lines()` gives mailpiece
   order.
3. **`#` for an unknown designator.** Confirmed: `# 4` with a space
   (213.2), and no `#` when the designator is known (213), so `APT #4` is
   `APT 4`.
4. **Last line.** Confirmed: `CITY ST ZIP` with at least one space (224;
   two preferred between state and ZIP, one used), no comma (222), ZIP+4
   hyphen kept. City names: spelled out in full, abbreviated only from the
   City State file (223), so no SAINT/ST or FORT/FT rule is applied.
5. **State inside a street name.** Implemented in the parser and both
   emitters from Appendix F: a state code or name before a highway prefix
   (`CA COUNTY RD 150`, `KY ST HWY 1`), or before a route number alone
   (`KENTUCKY 440`, `KY 1207`, which take the HIGHWAY prefix), is kept as
   `Highway::state` and written as its code. A state that is the whole
   primary name (`OKLAHOMA AVE`) stays spelled out. Corpus
   `p28-highway-028` to `p28-highway-037`.
6. **Leading zeros.** Decided: keep route numbers as given (see the table
   above). Stripping them only in the emitter would make the USPS output key
   differently from its input; stripping them in the parser or key would
   change `k1` keys for those inputs. Revisit with any `k2`.
7. **Military and general delivery.** Confirmed against the 238.1 and 239
   examples (`CPR 895 BOX 74` / `APO AE 09499-0001`,
   `UNIT 9900 BOX 0500` / `DPO AE 09701-0500`) and section 261. The
   corpus military and general delivery cases now check `usps`.
8. **Key punctuation rule.** Traced: `docs/key-format.md` rule 1 now cites
   sections 222 and 354.
9. **Release re-check (done 2026-10-17).** Every lexicon table and the
   emitter rules above were re-checked against the live Pub 28 pages
   (`docs/design.md`, item 44); see "Re-checked 2026-10-17" under
   "Verification status". Two bugs found and fixed (section 234.2 and letter
   routes).

Special forms (sourced):

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
- Highways (section 237, Appendix F): emit `[<state code>] <standard prefix> <route>`, for
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
