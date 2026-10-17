# tools/calibrate

Tools for the hand-labeled Florida sample that calibrated confidence is
fitted on (`docs/design.md` Milestone 7, items 36 and 38). The sample and
everything derived from it stay in `local-data/florida-sample/`; nothing
under that folder is committed.

| Tool | What it does |
|---|---|
| `sample_fl.py` | Draws the sample (seeded) and writes draft records pre-filled with the current parse; `--status` counts what is left |
| `label.py` | Labels rows one at a time in the terminal (the author's tool) |
| `triage.py` | Sorts unlabeled rows into clear and review, accepts or applies labels in bulk (collaborative mode, below) |
| `fit.py` | Fits the confidence score (item 38) on the corpus, usaddress's test files and the Florida sample; `--report` writes the calibration report, `--out` the fitted model as TOML |

Each record's `note` says who labeled it: no marker for the author (via
`label.py`), `by=assistant` for a draft the assistant checked and accepted,
`by=assistant+author` for an assistant recommendation the author approved.

## Collaborative mode

The author and an assistant (Claude Code) label together: the assistant
clears the rows that are plainly right and proposes labels for the rest; the
author decides every row that needs judgment. It can stop after any step and
resume at the first one, because each step reads the sample's current
state.

### Files (in `local-data/florida-sample/`)

| File | Written by | Holds |
|---|---|---|
| `sample.jsonl` | all tools | the sample; a row is unlabeled while its note ends in `TODO` |
| `TRIAGE.md` | `triage.py` | unlabeled rows, clear or review with the reasons |
| `recommendations.ndjson` | the assistant | one proposal per row (format below); `.ndjson` so the corpus runner does not load it |
| `REVIEW.md` | `triage.py` | the proposals for rows still unlabeled, by group, for the author |
| `NOTES.md` | `label.py`, `triage.py --apply` | one line per row where the parser is wrong |

### Steps

1. **Triage.** `python3 tools/calibrate/triage.py`. A row is *clear* only
   when an independent walk of its input accounts for every word, in address
   order, with the draft's values; the city is a known place matched by name;
   there are no warnings; and nothing needs judgment (a bare unit, a street
   with no suffix, a highway word in a name, an attention line, unparsed
   text). The walk does not trust the parser. Everything else is *review*.
2. **The assistant reads every clear row** in `TRIAGE.md` and accepts only
   those it would label the same way independently:
   `python3 tools/calibrate/triage.py --accept ID...` (`by=assistant`). A
   clear row that looks wrong is not accepted; it gets a recommendation in
   step 3, and the check that missed it is added to `triage.py`.
3. **The assistant writes a recommendation for each review row** in
   `recommendations.ndjson`, then reruns step 1, which renders `REVIEW.md`.
   Each recommendation is checked by the corpus runner when applied.
4. **The author reviews `REVIEW.md`**, group by group:
   - approve: `python3 tools/calibrate/triage.py --apply ID...`, or a whole
     group: `--apply ALL --group accept` (`by=assistant+author`; a proposal
     with `parser_error` adds its `NOTES.md` line);
   - overrule: `python3 tools/calibrate/label.py --start ID` and label it by
     hand.
5. **Spot-check.** `python3 tools/calibrate/label.py --review` walks the
   labeled rows; `cargo run -q -p addr-cli -- corpus test --dir
   local-data/florida-sample --verbose` shows every row that differs from
   the parser (each should be in `NOTES.md`).

Applied and accepted rows refuse to be written again (`already labeled`), so
repeating a command is safe. To restart from any point, run step 1.

### `recommendations.ndjson` format

One JSON object per line:

```json
{"id": "fs-042", "group": "change", "expect": {...}, "warnings": [...],
 "why": "what the right reading is and why",
 "parser_error": "one line for NOTES.md, or null",
 "ambiguous": "reason for an ambiguous: note, or null"}
```

`group` is one of `change` (the draft is wrong), `your call` (judgment
needed), `ambiguous` (draft right, marked ambiguous), `accept` (draft right;
held back only by a warning the conventions require). `expect` and
`warnings` follow `tests/corpus/README.md`; `warnings` uses the same
computed rules as `label.py`.

### Rules for the assistant

- Label from the input and the corpus conventions, never from the parser's
  output; the draft is a convenience, not a source.
- Accept nothing it would not have written itself; when unsure, recommend
  and let the author decide.
- Never copy rows out of `local-data/`, and never repeat a person's name in
  chat or in committed files (an attention line may hold one).

## Fitting the score (`fit.py`)

```sh
cargo build --release -p addr-cli
python3 tools/calibrate/fit.py --sample-addr PRE_FIX/addr \
    --report local-data/reports/confidence-fit.md [--out FITTED.toml]
```

The label is "the whole parse is right"; the features are the tagger's
decisions and the warnings, read from the binary (`addr corpus features`,
`addr parse --features`), so no rule is re-implemented in Python.

- **Sample, before the fixes.** Item 36's commit 4 fixed seven rules from the
  sample's errors. `--sample-addr` is an `addr` built from the commit before
  those fixes (`git worktree add ../pre 51d8d5c`, apply the export patch of
  item 38 commit 1, `cargo build --release`), so the sample is scored as the
  parser was when it was labeled. Without it the sample looks better than
  it was.
- **Sampling.** The sample over-draws flagged rows. That does not bias a
  fit conditional on the warnings, because the warnings are the features.
- **Weights.** The Florida sample carries half the weight
  (`--florida-share`); the synthetic gold files would otherwise decide the
  baseline.
- **Constraint.** A warning or decision never raises confidence. The
  unconstrained fit gave `missing_state` a large positive coefficient only
  because the synthetic files lack states and are rarely wrong.
- **The geocode benchmark is not read here.** It checks the result (item 38,
  commit 3).
