# CLAUDE.md

Rust-core US address parser with a PyO3 Python wrapper. Read docs/design.md first.

## Rules
- addr-core must stay pure Rust with no Python dependency. PyO3 lives only in addr-py.
- Never panic on input. Parsing always returns a ParsedAddress with diagnostics.
- Emitters read ParsedAddress; they never re-parse or mutate it.
- All lexicon access goes through the Lexicons interface. No state-specific logic in the tagger.
- The canonical key format is a public contract. Any change requires a new version prefix and a docs/key-format.md update.
- Every USPS or RESO rule implemented must be traceable to a source in docs/.
- This library formats and parses; it does not validate deliverability. Do not imply otherwise in docs or code comments.

## Workflow
- Run `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test --all-features` before finishing a task.
- Add a golden-corpus case for every parsing bug fixed (format: tests/corpus/README.md). When a change makes more cases pass, rerun the corpus test with `UPDATE_CORPUS_BASELINE=1` and commit `tests/corpus/passing.txt`.
- Work from the plan in docs/design.md section 10: one pull request per numbered item, in order, with small commits. Put newly found work into section 10 if scheduled, otherwise into the section 13 backlog.
- Python: `maturin develop` then `pytest crates/addr-py/tests`.
