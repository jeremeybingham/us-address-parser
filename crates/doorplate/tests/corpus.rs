//! Golden-corpus test. The runner is `doorplate::corpus` (also behind
//! `doorplate corpus test`); format and conventions: `tests/corpus/README.md`.
//!
//! The corpus may be ahead of the parser, so a failing case alone does not
//! fail the test. It fails when the corpus is malformed, or when a case
//! listed in `tests/corpus/passing.txt` no longer passes. Run with
//! `UPDATE_CORPUS_BASELINE=1` to rewrite that file from the current results,
//! and `CORPUS_VERBOSE=1` to print the failing cases.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use doorplate::corpus;
use doorplate::Parser;

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus")
}

/// The published package has no `tests/corpus`; these tests then do nothing.
fn have_corpus() -> bool {
    corpus_dir().is_dir()
}

fn load() -> corpus::Corpus {
    match corpus::load(&corpus_dir()) {
        Ok(c) => c,
        Err(errors) => panic!("malformed corpus:\n  {}", errors.join("\n  ")),
    }
}

#[test]
fn golden_corpus() {
    if !have_corpus() {
        return;
    }
    let dir = corpus_dir();
    let report = load().run(&Parser::new());
    print!("{}", report.summary());
    if std::env::var_os("CORPUS_VERBOSE").is_some() && !report.failure_details().is_empty() {
        println!("failing cases:\n{}", report.failure_details());
    }
    if std::env::var_os("UPDATE_CORPUS_BASELINE").is_some() {
        let path = corpus::write_baseline(&dir, &report).unwrap();
        println!("wrote {}", path.display());
        return;
    }
    let cmp = report.compare(&corpus::read_baseline(&dir));
    if !cmp.newly_passing.is_empty() {
        println!(
            "{} case(s) now pass but are not in passing.txt; run with UPDATE_CORPUS_BASELINE=1 to record them: {:?}",
            cmp.newly_passing.len(),
            cmp.newly_passing
        );
    }
    assert!(
        cmp.stale.is_empty(),
        "passing.txt lists ids that are not in the corpus: {:?}",
        cmp.stale
    );
    let diffs: Vec<_> = cmp
        .regressed
        .iter()
        .filter_map(|id| report.failure(id))
        .collect();
    assert!(
        cmp.regressed.is_empty(),
        "regressions: these cases are in passing.txt but now fail:\n{}",
        diffs.join("\n")
    );
}

/// Parsing the USPS output of a parsed address must give the same keys.
#[test]
fn usps_output_round_trips_to_the_same_key() {
    if !have_corpus() {
        return;
    }
    let bad = load().usps_round_trip_mismatches(&Parser::new());
    println!("usps round trip: {} mismatch(es)", bad.len());
    assert!(
        bad.is_empty(),
        "round-trip mismatches:\n  {}",
        bad.join("\n  ")
    );
}
