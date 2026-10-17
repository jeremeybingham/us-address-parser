# Contributing to doorplate

Thanks for helping. The most useful contribution is usually a **misparsed address**.

## Reporting a misparse

Open an issue with:

- the input exactly as you gave it (one line, or each field),
- what you expected for each component, and what `doorplate parse "<input>" --pretty`
  printed (the version is in the output),
- where the shape comes from, if you know (a county list, a form), so similar cases
  can be found.

Please use invented or public addresses. Do not paste personal data.

## Building and testing

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Python (in a virtualenv with maturin and pytest)
cd crates/doorplate-py && maturin develop --release && cd ../..
pytest crates/doorplate-py/tests
```

Rust 1.80 or later; the Python extension needs 1.83. Plain `cargo test` needs no Python.

## Fixing a parsing bug

1. Add a case to the golden corpus (`tests/corpus/`, format in
   `tests/corpus/README.md`) that fails. Use an invented address of the same shape.
2. Fix the rule or the table. The tagger reads tables only through `Lexicons`; there
   is no state-specific logic in it.
3. Add a row to `docs/usps-pub28-notes.md` for a new parser rule, with its source: a
   Pub 28 section, or the shape you observed.
4. Run the corpus with `UPDATE_CORPUS_BASELINE=1` if more cases now pass, and commit
   `tests/corpus/passing.txt`. Run `python3 tools/corpus/emit_expect.py --check
   tests/corpus/*.jsonl`.
5. If the fix changes the key an input produces, it is a key-format change: it needs a
   new key version and an update to `docs/key-format.md`. Say so in the pull request.

## Data changes

Tables live in `crates/doorplate/data/`. A spelling that Pub 28 does not list goes
in an `observed` list with its evidence and raises `non_standard_abbreviation`. A
state pack addition names its evidence and is add-only (`docs/packs/fl.md`). A change
to a Pub 28 table should be checked with `tools/pub28/verify.py`.

## Rules for the code

- `doorplate` stays pure Rust with no Python dependency; emitters never re-parse or
  change a parse; parsing never panics.
- This library formats and parses; it does not validate deliverability. Keep docs and
  comments consistent with that.
- Small commits, one change per pull request, with a short explanation of why.

## License

By contributing you agree that your work is licensed under the project's terms:
MIT OR Apache-2.0, at the user's option.
