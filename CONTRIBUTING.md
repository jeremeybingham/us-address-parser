# Contributing to doorplate

Thanks for helping. The most useful contribution is usually a **misparsed address**.

## Reporting a misparse

Open an issue with:

- the input exactly as you gave it (one line, or each field),
- what you expected for each component, and what `doorplate parse "<input>" --pretty`
  printed,
- the version (`python -c "import doorplate; print(doorplate.__version__)"` or the crate
  version),
- where the shape comes from, if you know (a county list, a form), so similar cases
  can be found.

Please use invented or public addresses. Do not paste personal data.

## Sharing messy data

The most valuable thing you can give this project is real, messy addresses: the kind
that come from county exports, mailing lists, forms and old databases. If a sample
parses badly, or you are not sure it parses well, open an issue (the template asks for
the right details) and attach what you can.

If the data is private, anonymize it first; an anonymized sample is just as useful as
long as it keeps the *shape* of each address:

- keep the abbreviations, punctuation, spacing, case, typos and the order of the parts;
- replace personal and business names, and house numbers, with invented ones of the same
  form (`4417` can become `1305`, `A12` can become `B27`);
- replace street and city names with other real or invented names of similar length and
  structure, but keep the words that affect parsing: suffixes (`ST`, `LN`), directionals,
  unit designators, and place names that contain suffix words (`MOUNTAIN VIEW`);
- keep the state, and a ZIP of the same state, when the parse depends on them;
- never send anything that could identify a person.

If you cannot share even that, say so in the issue and describe the shapes in words
(for example "unit written before the street, then the city cut off at 30 characters");
we can work out an anonymization together. You can also send counts: how many of your
rows come back with each warning.

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
