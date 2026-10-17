//! Parse and emit benchmarks: `cargo bench -p doorplate --bench parse`.
//!
//! Inputs are the golden corpus (`tests/corpus/*.jsonl`), so the numbers
//! track the shapes the library is tested on. Speed is not the library's
//! bar (accuracy is, `docs/design.md` section 2); these are here to catch
//! regressions and to size bulk jobs. Run with `--features rayon` to see
//! the parallel `parse_many`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::hint::black_box;
use std::path::Path;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use doorplate::{corpus, Fields, KeyOptions, Parser};

/// One address per common shape, from the corpus README's examples.
const SHAPES: [(&str, &str); 6] = [
    ("street_only", "1305 SUNWOOD DR"),
    ("full_one_line", "123 N Main St Apt 4, Miami, FL 33101"),
    (
        "unit_and_zip4",
        "4158 INVERRARY DR #309, LAUDERHILL FL 33319-1234",
    ),
    ("po_box", "PO BOX 1234, MIAMI FL 33101"),
    ("highway", "8932 HWY 2301, DEFUNIAK SPRINGS FL 32433"),
    ("military", "PSC 1650 BOX 10, APO AE 09001"),
];

fn corpus_inputs() -> Vec<String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus");
    corpus::load(&dir)
        .unwrap_or_else(|e| panic!("malformed corpus:\n  {}", e.join("\n  ")))
        .inputs()
}

fn parse_single(c: &mut Criterion) {
    let parser = Parser::new();
    let mut g = c.benchmark_group("parse_single");
    for (name, input) in SHAPES {
        g.bench_function(name, |b| b.iter(|| parser.parse(black_box(input))));
    }
    let fields = Fields {
        line1: Some("4158 INVERRARY DR #309".into()),
        city: Some("LAUDERHILL".into()),
        state: Some("FL".into()),
        zip: Some("33319".into()),
        ..Fields::default()
    };
    g.bench_function("fields", |b| {
        b.iter(|| parser.parse_fields(black_box(&fields)))
    });
    g.finish();
}

fn parse_corpus(c: &mut Criterion) {
    let parser = Parser::new();
    let inputs = corpus_inputs();
    let refs: Vec<&str> = inputs.iter().map(String::as_str).collect();
    let mut g = c.benchmark_group("parse_corpus");
    g.throughput(Throughput::Elements(refs.len() as u64));
    g.bench_function("one_by_one", |b| {
        b.iter(|| {
            for s in &refs {
                black_box(parser.parse(black_box(s)).ok());
            }
        })
    });
    g.bench_function("parse_many", |b| {
        b.iter(|| parser.parse_many(black_box(&refs)))
    });
    g.finish();
}

fn emit(c: &mut Criterion) {
    let parser = Parser::new();
    let a = parser
        .parse("123 N Main St Apt 4, Miami, FL 33101")
        .unwrap();
    let mut g = c.benchmark_group("emit");
    g.bench_function("to_usps", |b| b.iter(|| black_box(&a).to_usps()));
    g.bench_function("to_reso", |b| b.iter(|| black_box(&a).to_reso()));
    g.bench_function("key", |b| {
        b.iter(|| black_box(&a).key(KeyOptions::property()))
    });
    g.bench_function("slug", |b| b.iter(|| black_box(&a).slug()));
    g.finish();
}

criterion_group!(benches, parse_single, parse_corpus, emit);
criterion_main!(benches);
