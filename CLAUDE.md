# CLAUDE.md

Rust-core US address parser with a PyO3 Python wrapper. Read docs/how-it-works.md first.

## Rules
- doorplate must stay pure Rust with no Python dependency. PyO3 lives only in doorplate-py.
- Never panic on input. Parsing always returns a ParsedAddress with diagnostics.
- Emitters read ParsedAddress; they never re-parse or mutate it.
- All lexicon access goes through the Lexicons interface. No state-specific logic in the tagger.
- The canonical key format is a public contract once released. Until the first release a parser fix may change a key, but log it (dated) in docs/key-format.md; after release any change requires a new version prefix (docs/design.md decision 34).
- Every USPS or RESO rule implemented must be traceable to a source in docs/.
- This library formats and parses; it does not validate deliverability. Do not imply otherwise in docs or code comments.

## Workflow
- Run `cargo fmt`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace --all-features` before finishing a task.
- Add a golden-corpus case for every parsing bug fixed (format: tests/corpus/README.md). When a change makes more cases pass, rerun the corpus test with `UPDATE_CORPUS_BASELINE=1` and commit `tests/corpus/passing.txt`.
- Work from the plan in docs/design.md section 10: one pull request per numbered item, in order, with small commits. Put newly found work into section 10 if scheduled, otherwise into the section 13 backlog.
- Python: in `crates/doorplate-py`, `maturin develop`, then `pytest crates/doorplate-py/tests`.
- Parse-rule changes also need a row in docs/usps-pub28-notes.md (its source, even when the source is data rather than USPS). Re-run `python3 tools/corpus/emit_expect.py --check tests/corpus/*.jsonl` after changing a rule the corpus emitters follow.
- Private data (DBPR exports, the geocode benchmark) stays in `local-data/`; never commit it or print licensee names.
- Text-only cleanup for the release (build-process wording, naming the private data sources, README tone, old names) is deferred: add it to `docs/repository_cleaning.md`, do not apply it piecemeal.
