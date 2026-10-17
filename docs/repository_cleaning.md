# Repository cleaning before the first release

Every change to the repository's **text** that waits for the fresh start,
in one place (decision 42 in `docs/design.md`). Nothing here is done piecemeal
while development continues: the text still serves the build, and the
history is wiped anyway. It is applied as one pass (item 61), checked, and
then copied into a new repository with no history (item 62), from which
the first release is published (item 46).

When a new cleaning item comes up during development, add it here, not to
the plan or the backlog (`CLAUDE.md`).

## When

After the code for the first release is final: items 44, 57,
58 and 45, and whichever Milestone 9 items ship in the first release
(section 13, open decisions). Code changes after the cleaning pass mean
re-checking the sections below for the files they touch.

## What the released text should be

Documentation of what the library does and how to use it. The record of how
it was built shrinks to a short historical outline in `docs/design.md`. A
reader who has never seen this repository should find, in order: what it
is for, how to install it, how to use it, what it outputs, how accurate and
fast it is, and how to customize it.

## 1. Build-process language

Today's text narrates the build: milestone and item numbers, decision
numbers, "done", "next", "planned", pull-request rules. A rough count of
`Milestone N`, `item N` and `decision N` references: `docs/design.md` 151,
`CHANGELOG.md` 40, `docs/usaddress-comparison.md` 17, `docs/packs/fl.md` 8,
`docs/key-format.md` 8, `crates/addr-core/src/types.rs` 8, and a few in each
other doc, source file and tool.

- [ ] `docs/design.md` becomes a historical outline: goals and scope, the
      architecture and why, the decisions that still constrain the code
      (each in a line or two, without numbers), and a short history of the
      milestones. The section 10 plan, item numbers, open decisions and
      backlog go; open work moves to GitHub issues in the new repository.
- [ ] Source comments and doc comments: drop item, milestone and decision
      numbers (`(item 13)`, `decision 12`); keep the reason a rule exists and
      its source (Pub 28 section, data table).
- [ ] Other docs (`docs/key-format.md`, `docs/reso-mapping.md`,
      `docs/places.md`, `docs/packs/fl.md`, `docs/usps-pub28-notes.md`,
      `tests/corpus/README.md`, `crates/addr-py/README.md`): describe current
      behavior only. Remove "until item N", "(done)", dated change narration
      except where it is a real contract (the key change log in
      `docs/key-format.md` collapses to "k1, as of the first release").
- [ ] `CHANGELOG.md`: replace the development log with one entry for the
      first release (features, not history).
- [ ] `CLAUDE.md`: rewrite for maintenance after release (key policy, checks,
      corpus rule, the source rule); drop the one-PR-per-plan-item workflow.
- [ ] Tool docstrings and `README`s under `tools/`: what each tool does and
      how to run it, without milestone context; decide which tools ship
      (section 3).

## 2. Naming the data sources

The Florida address data used for testing and for the Florida pack is public
record and fine to use, but the released text should not name where it came
from. Mentions today (case-insensitive `dbpr`, `auction`, `licensee`,
`clerk`, `tax deed`): `tests/corpus/regressions.jsonl` 61,
`docs/usps-pub28-notes.md` 29, `docs/packs/fl.md` 23, `docs/design.md` 20,
`CHANGELOG.md` 18, `tools/calibrate/sample_fl.py` 18, `fl-pack.jsonl` 12,
`tests/corpus/README.md` 11, `docs/prior-art.md` 9,
`docs/usaddress-comparison.md` 8, `data/*.toml` 17 together,
`data/packs/fl/` 6, the `tools/packs/` scripts, `NOTICE`, `CLAUDE.md`, and
three source comments.

- [ ] Replace the names with a generic description: "Florida public-record
      address lists" or "observed in Florida address data". Keep the facts
      that justify a rule (how many rows, what spelling) where they matter.
- [ ] Rules stay traceable (`CLAUDE.md`): every USPS or RESO rule keeps its
      Pub 28 or Data Dictionary citation; a rule taken from data cites "Florida
      address data (observed)" in `docs/usps-pub28-notes.md`.
- [ ] Corpus: case `note`s and the source sections of
      `tests/corpus/README.md`; rename the `at-` and `atg-` id prefixes
      (they name auction-tracker) and regenerate `passing.txt`. Case ids are
      not a public contract.
- [ ] `docs/prior-art.md`: either generalize ("an earlier Florida address
      parser") or fold its lessons into `docs/design.md` and drop it.
- [ ] `data/packs/fl/pack.toml` sources list and `data/*.toml` comments.
- [ ] `tools/packs/dbpr.py`, `observe_fl_zips.py`, `measure_fl_places.py`,
      `tools/calibrate/sample_fl.py` and `label.py`: rename to generic names
      or leave them out of the new repository (they need the private files);
      `local-data/` is never copied.
- [ ] `NOTICE`: the public-record data needs no attribution; drop the named
      line or describe it generically. Keep and check every attribution that
      is required:
  - GeoNames postal codes (CC BY 4.0): attribution required; keep the
    "reformatted" statement.
  - US Census Gazetteer and reference files: public domain; credit kept as
    courtesy.
  - USPS Publication 28 and the RESO Data Dictionary: cited as the
    standards followed (no data copied beyond the tables of abbreviations).
  - usaddress test files (MIT, datamade), vendored in
    `tests/external/usaddress/`: keep the license text with them.
  - The geocode benchmark and Netsyms data are never committed; cite the
    paper and the Netsyms site where results are reported, with no data.
- [ ] The Florida pack's own `cities_fl.json` source (the author's code,
      MIT OR Apache-2.0): describe as "curated by the authors".

## 3. README and user docs

- [ ] `README.md` rewritten for a first-time user: one paragraph on what it
      does and for whom; install (Rust, Python, CLI); a 10-line example per
      interface; the output fields; the key and slug in one paragraph each;
      accuracy and speed in a few numbers with a link (section 4); what it
      does not do (it formats and parses; it does not validate
      deliverability); customization (link to the custom-layer guide, item
      60); examples (item 59); license and attribution.
- [ ] Diagrams (ASCII or Markdown/Mermaid), in the README or a
      `docs/how-it-works.md`: the pipeline (clean, tokenize, classify, tag,
      assemble, place, emit); how the last line is read right to left and
      the delivery line from both ends; the lexicon layers (base tables,
      national places, state packs, custom layer) behind `Lexicons`; one
      address traced through to its key, USPS and RESO forms.
- [ ] `crates/addr-py/README.md`: the same treatment for the Python package
      (PyPI shows it).
- [ ] Docs index: `docs/README.md` or a table in the README listing the
      remaining docs by purpose (reference, data, contracts).
- [ ] Drop or move docs that only served the build: `docs/future-sources.md`
      becomes a short "data sources considered" note or goes; the CRF and
      model history stays only in the design outline.

## 4. Tone toward usaddress

usaddress is a related tool that does something similar and something
different, not a competitor. Today it is mentioned 67 times in
`docs/design.md`, 46 in `docs/usaddress-comparison.md`, 29 in `CHANGELOG.md`,
19 in `README.md`, 12 in `docs/usps-pub28-notes.md`, and in the Python
README and corpus docs.

- [ ] Keep: the benchmark numbers and their method, the feature-parity table,
      the `tag()` / `--usaddress` compatibility output and its label mapping,
      the migration notes ("Coming from usaddress"), and the attribution of
      its test files.
- [ ] Remove: "against usaddress" framing in the status and summaries,
      "wins/loses" language, and comparison as a motivation for features.
      Features are described by what they do.
- [ ] Rename `docs/usaddress-comparison.md` to `docs/benchmarks.md`, built
      around item 57's results (accuracy on the blind benchmark and the
      corpus, speed in Rust and Python, memory and binary size), with
      usaddress as one reference point among the measurements.

## 5. Names

The rename itself is item 45, with the name chosen in item 58. After it:

- [ ] No text, path, badge, URL or example uses an old name (`addr-core`,
      `addr-cli`, `addr-py`, `us_address_parser`, `us-address-parser`,
      `us-addr-*`) unless on purpose (a "formerly" line is not needed: there
      was no release).
- [ ] The new repository name matches the package names.

## 6. The fresh repository (item 62)

- [ ] Sections 1 to 5 done and reviewed on the old repository (one pull
      request per section is fine; they are text only).
- [ ] Tests, clippy, the corpus, fuzz smoke, the Python suite and the wheel
      builds all green on the final tree.
- [ ] Decide what is copied: everything tracked, minus tools and files
      section 2 leaves out; never `local-data/`.
- [ ] Scan the tree for private data before the copy: no licensee or other
      personal names, no rows from private files beyond the corpus
      addresses already reviewed, no secrets or tokens, no local paths.
- [ ] Create the new repository (renamed), commit the tree as one initial
      commit, set up CI, branch protection and the publishing secrets there.
- [ ] Archive the old repository (private) rather than delete it, so the
      development history stays available to the author.
- [ ] Publish from the new repository (item 46); `k1` freezes there.

## 8. Confidence is deferred (decision 43)

The first release ships `confidence = None`. Text and code that promise or
imply a score, to settle in the cleaning pass:

- [x] README status block (facts corrected 2026-10-17; tone stays for the pass): no "then calibrated confidence"; say `confidence`
      is reserved and always `None` (`README.md`, `crates/addr-py/README.md`,
      the `confidence` docstrings in `types.rs` and `addr-py/src/lib.rs`, the
      `.pyi` stub has no docstring to change, `docs/usaddress-comparison.md`
      feature table).
- [x] `docs/future-sources.md`: "confidence (still `None`; item 36 and
      Milestone 7)" and the optional NAD calibration step point to the
      backlog entry "Calibrated confidence" instead of item 38.
- [ ] `CHANGELOG.md`: item 37's "will be fitted on them (item 38)" line stays
      historical; the release notes say `confidence` is reserved.
- [ ] `tools/calibrate/`: decide what the fresh repository keeps. The
      labeling tools (`sample_fl.py`, `label.py`, `triage.py`) work on the
      private Florida sample and name its sources; `fit.py` is generic.
      Recommend keeping `fit.py` and its README section as the starting
      point for the deferred work, and moving the sample tools out or
      renaming them (section 3).
- [ ] The tagger's decision export (`Diagnostics` internal field,
      `decision_codes`, `addr corpus features`, `addr parse --features`,
      `Corpus::features`) is groundwork for the later fit. It is hidden from
      the docs and behind the `corpus` feature; keep it unless the pass
      finds it confusing.
- [ ] `docs/benchmarks.md` (item 57) has no calibration section; if it is
      wanted later it comes with the backlog entry.
- [x] The Python test `test_absent_components_are_none_and_confidence_is_reserved` (renamed)
      keeps asserting `None`.

