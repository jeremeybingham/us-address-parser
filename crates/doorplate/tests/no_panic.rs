//! Property tests for the never-panic rule (CONTRIBUTING.md): any input parses
//! without panicking, every span indexes valid text, and every emitter runs.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use doorplate::{Fields, KeyOptions, ParseError, ParseOptions, ParsedAddress, Parser};
use proptest::prelude::*;

/// Address-shaped fragments, so generated input reaches the tagger rules
/// rather than stopping at the first unknown token.
const WORDS: &[&str] = &[
    "123",
    "N",
    "SW",
    "Main",
    "St",
    "Street",
    "Apt",
    "Unit",
    "#",
    "4B",
    "PO",
    "Box",
    "RR",
    "HC",
    "Hwy",
    "US",
    "Highway",
    "I-95",
    "FL",
    "Miami",
    "33101",
    "33101-1234",
    "APO",
    "AE",
    "Gen",
    "Delivery",
    "1/2",
    "12-14",
    "No",
    "Situs",
    "St.",
    "Ste",
    "Fl",
    "Rd",
    "Ave",
    "&",
    "-",
    ",",
    ";",
    ".",
    "é",
    "ß",
    "İ",
    "日本",
    "\u{0}",
    "\t",
    "\u{200b}",
    "\u{202e}",
];

fn address_like() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(WORDS), 0..14).prop_map(|w| w.join(" "))
}

fn field() -> impl Strategy<Value = Option<String>> {
    prop::option::of(prop_oneof![address_like(), any::<String>()])
}

fn check(r: Result<ParsedAddress, ParseError>) {
    let Ok(a) = r else { return };
    for cs in &a.source.spans {
        let text = a.source.text(&cs.span);
        assert!(text.is_some(), "span does not index input: {cs:?}");
        assert!(cs.span.start <= cs.span.end);
    }
    let _ = (a.key(KeyOptions::property()), a.key(KeyOptions::unit()));
    let _ = a.slug();
    let _ = a.to_usps();
    let _ = a.to_usps_wrapped(30);
    let _ = a.to_reso().fields();
    let _ = a.to_reso().length_overruns();
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    #[test]
    fn arbitrary_string_never_panics(s in any::<String>()) {
        let p = Parser::new();
        check(p.parse(&s));
    }

    #[test]
    fn address_like_never_panics(s in address_like()) {
        let p = Parser::new();
        check(p.parse(&s));
    }

    #[test]
    fn fields_never_panic(
        line1 in field(), line2 in field(), city in field(),
        state in field(), zip in field(),
    ) {
        let p = Parser::new();
        let f = Fields { line1, line2, city, state, zip };
        check(p.parse_fields(&f));
    }

    /// A default_state hint never panics and never fills the state.
    #[test]
    fn hinted_parser_never_panics_or_fills_state(
        s in address_like(), st in prop::sample::select(&["FL", "AL", "ny", "Texas", "DC", "PR"][..]),
    ) {
        let hinted = Parser::with_options(ParseOptions {
            default_state: Some(st.to_string()),
        }).unwrap();
        let plain = Parser::new();
        if let (Ok(a), Ok(b)) = (hinted.parse(&s), plain.parse(&s)) {
            prop_assert_eq!(a.state, b.state);
            prop_assert_eq!(a.zip5.clone(), b.zip5.clone());
            // Only the hint's own reading differs; with a state or a known
            // ZIP the two parses are identical.
            if b.state.is_some() {
                prop_assert_eq!(a, b);
            }
        }
        check(hinted.parse(&s));
    }

    #[test]
    fn parse_many_matches_parse(v in prop::collection::vec(address_like(), 0..8)) {
        let p = Parser::new();
        let refs: Vec<&str> = v.iter().map(String::as_str).collect();
        let many = p.parse_many(&refs);
        prop_assert_eq!(many.len(), refs.len());
        for (m, s) in many.into_iter().zip(&refs) {
            prop_assert_eq!(m, p.parse(s));
        }
    }

    /// Keys are spelling-independent: case changes never change the key.
    #[test]
    fn key_ignores_ascii_case(s in address_like()) {
        let p = Parser::new();
        if let (Ok(a), Ok(b)) = (p.parse(&s), p.parse(&s.to_uppercase())) {
            if s.is_ascii() {
                prop_assert_eq!(a.key(KeyOptions::unit()), b.key(KeyOptions::unit()));
            }
        }
    }
}
