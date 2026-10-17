# CLAUDE.md

Rust-core US address parser (`doorplate`) with a PyO3 Python wrapper. Read
`docs/how-it-works.md` first.

## Rules
- `doorplate` must stay pure Rust with no Python dependency. PyO3 lives only in `doorplate-py`.
- Never panic on input. Parsing always returns a `ParsedAddress` with diagnostics.
- Emitters read `ParsedAddress`; they never re-parse or mutate it.
- All lexicon access goes through the `Lexicons` interface. No state-specific logic in the tagger.
- The canonical key format is a public contract. A change to what an input produces needs a new version prefix (`k2`) and an update to `docs/key-format.md`; never edit `k1` in place.
- Every USPS or RESO rule implemented must be traceable to a source in `docs/` (`docs/usps-pub28-notes.md`, `docs/reso-mapping.md`).
- This library formats and parses; it does not validate deliverability. Do not imply otherwise in docs or code comments.

## Workflow
- Before finishing a task run `cargo fmt`, `cargo clippy --workspace --all-targets --all-features -- -D warnings` and `cargo test --workspace --all-features`.
- Add a golden-corpus case for every parsing bug fixed (format: `tests/corpus/README.md`). When a change makes more cases pass, rerun the corpus test with `UPDATE_CORPUS_BASELINE=1` and commit `tests/corpus/passing.txt`.
- Parse-rule changes also need a row in `docs/usps-pub28-notes.md` (its source, even when the source is data rather than USPS). Re-run `python3 tools/corpus/emit_expect.py --check tests/corpus/*.jsonl` after changing a rule the corpus emitters follow.
- After changing a table in `crates/doorplate/data/`, run `python3 tools/pub28/verify.py --cache DIR --fetch --emitter` (needs the built Python extension) to compare with the live Pub 28 pages.
- Python: in `crates/doorplate-py`, `maturin develop`, then `pytest crates/doorplate-py/tests`.
- Private data (local exports, the geocode benchmark) stays in `local-data/` (git-ignored); never commit it, and never print personal names from it.
- Keep small commits and one change per pull request.
