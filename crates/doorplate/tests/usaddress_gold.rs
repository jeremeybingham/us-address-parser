//! Score `ParsedAddress::to_usaddress_tags` against usaddress's labeled test
//! files (`tests/external/usaddress/`), with the coarse rules of
//! `docs/usaddress-comparison.md`, and fail if the score drops below the
//! recorded floor. The full two-parser comparison is
//! `tools/compare/compare_usaddress.py`; this test needs no Python.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

use doorplate::Parser;

/// Recorded scores (2026-10-15, after the benchmark rule fixes; 99.76% and
/// 99.46%), rounded down. Raise them when the parser improves; never lower
/// them to make a change pass.
const TOKEN_FLOOR: f64 = 99.7;
const ADDRESS_FLOOR: f64 = 99.4;

const OUT_OF_SCOPE: [&str; 10] = [
    "Recipient",
    "LandmarkName",
    "CornerOf",
    "IntersectionSeparator",
    "NotAddress",
    "CountryName",
    "SubaddressType",
    "SubaddressIdentifier",
    "StreetNamePreModifier",
    "AddressNumberPrefix",
];

fn coarse(label: &str) -> &str {
    match label {
        "StreetNamePreDirectional"
        | "StreetNamePreType"
        | "StreetName"
        | "StreetNamePostType"
        | "StreetNamePostDirectional" => "StreetName",
        "AddressNumberSuffix" => "AddressNumber",
        other => other,
    }
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// One gold address: the input and its tokens as (byte start, byte end, label).
type GoldAddress = (String, Vec<(usize, usize, String)>);

/// The addresses of a usaddress training-format file, one per `<AddressString>`.
fn load(text: &str) -> Vec<GoldAddress> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(a) = rest.find("<AddressString>") {
        let body_start = a + "<AddressString>".len();
        let b = rest[body_start..].find("</AddressString>").unwrap() + body_start;
        let body = &rest[body_start..b];
        let (mut input, mut toks) = (String::new(), Vec::new());
        let mut k = 0;
        while k < body.len() {
            if body[k..].starts_with('<') {
                let close = body[k..].find('>').unwrap() + k;
                let label = &body[k + 1..close];
                let end_tag = format!("</{label}>");
                let text_end = body[close + 1..].find(&end_tag).unwrap() + close + 1;
                let tok = unescape(&body[close + 1..text_end]);
                let start = input.len();
                input.push_str(&tok);
                toks.push((start, input.len(), label.to_string()));
                k = text_end + end_tag.len();
            } else {
                let next = body[k..].find('<').map_or(body.len(), |n| n + k);
                input.push_str(&unescape(&body[k..next]));
                k = next;
            }
        }
        out.push((input, toks));
        rest = &rest[b..];
    }
    out
}

#[test]
fn usaddress_gold_scores_hold() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/external/usaddress");
    // The published package does not carry the vendored test files.
    if !dir.is_dir() {
        return;
    }
    let mut files: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "xml"))
        .collect();
    files.sort();
    assert!(
        !files.is_empty(),
        "no usaddress test files in {}",
        dir.display()
    );

    let parser = Parser::new();
    let (mut tokens, mut tok_ok, mut addrs, mut addr_ok) = (0usize, 0usize, 0usize, 0usize);
    for path in files {
        for (input, gold) in load(&fs::read_to_string(&path).unwrap()) {
            if gold
                .iter()
                .any(|(_, _, l)| OUT_OF_SCOPE.contains(&l.as_str()))
            {
                continue;
            }
            let tags = parser
                .parse(&input)
                .map(|a| a.to_usaddress_tags())
                .unwrap_or_default();
            let mut all = true;
            for (s, e, label) in &gold {
                if !input[*s..*e].chars().any(char::is_alphanumeric) {
                    continue;
                }
                let ours = tags
                    .iter()
                    .find(|t| t.span.start < *e && *s < t.span.end)
                    .and_then(|t| t.label);
                tokens += 1;
                if ours.map(coarse) == Some(coarse(label)) {
                    tok_ok += 1;
                } else {
                    all = false;
                }
            }
            addrs += 1;
            addr_ok += usize::from(all);
        }
    }
    let pct = |n: usize, d: usize| 100.0 * n as f64 / d as f64;
    let (tp, ap) = (pct(tok_ok, tokens), pct(addr_ok, addrs));
    println!("usaddress gold (coarse): tokens {tp:.2}% of {tokens}, addresses {ap:.2}% of {addrs}");
    assert!(
        tp >= TOKEN_FLOOR,
        "token accuracy {tp:.2}% fell below {TOKEN_FLOOR}%"
    );
    assert!(
        ap >= ADDRESS_FLOOR,
        "address accuracy {ap:.2}% fell below {ADDRESS_FLOOR}%"
    );
}
