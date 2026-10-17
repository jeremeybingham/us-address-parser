# Canonical key and slug spec (version k1)

This page specifies the **key** (below) and the **slug** (last section).

The key is a **public contract**. Stored keys must keep meaning the same thing,
so any change to what an input produces needs a new version prefix (`k2:`) and an
update here. `k1` is never edited in place.

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
  (`HIGHWAY 66 FRONTAGE`). `<suffix>` and the
  directionals are Pub 28 abbreviations.
- `M`: `<kind>` is CPR, OPC, PSC, UPR or UNIT; `<post office>` is APO, FPO or
  DPO (empty when absent).
- `<state>` is the USPS code. `<zip5>` is five digits.
- **Dual addresses:** a record with a street address and a PO Box (Pub 28
  section 215, `dual_address` warning) uses form `S`, from the street; the box is
  not in the key.
- **Floors:** a floor unit's value is a number (`5TH FLOOR` gives `5`).
- **Not in the key:** city, ZIP+4, unit type, the attention line, the
  building name, anything in
  `diagnostics`.
  - City is noisy and the field most likely to be corrected later; leaving it
    out keeps stored keys stable.
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
  give `4`, so `Apt 4` and `#4` match. The cost is that
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

## Stability

`k1` is the format as of the first release. The key is a function of the parse, and
the parse is a function of the shipped tables and rules: a bug fix that changes what
an input produces is a key change and needs `k2`. The slug and `place` are not part
of the key and may change with parser improvements.

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
  output all name the street (`9418-cuba-rd-hayes-va-23072`).
- Example: `123 N. Main Street, Apt 4, Miami FL 33101` gives
  `123-n-main-st-apt-4-miami-fl-33101`.

## Tests

`tests/corpus/keys.jsonl` pins exact key and slug strings (`key`, `key_unit`
and `slug` fields); unit tests in `crates/doorplate/src/emit/key.rs` and
`slug.rs` cover the rules.
