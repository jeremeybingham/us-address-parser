//! Public parser entry point (item 16).
//!
//! Pipeline: [`crate::clean`] -> [`crate::tokenize`] -> tagger
//! ([`crate::classify`] shapes and lexicon lookups) -> [`crate::assemble`].

use std::fmt;

use crate::assemble::assemble;
use crate::clean::clean;
use crate::input::Fields;
use crate::lexicon::Lexicons;
use crate::tagger::{tag_fields, tag_line, LineLengths};
use crate::tokenize::{tokenize, Token};
use crate::types::{FieldName, InputKind, ParsedAddress, Span};

/// Returned only for truly unusable input. Partial parses are never errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    EmptyInput,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::EmptyInput => f.write_str("address input is empty"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Address parser. Cheap to construct, `Send + Sync`, no global state.
#[derive(Debug, Clone, Default)]
pub struct Parser {
    lex: Lexicons,
}

impl Parser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse a single-line address. Commas (or semicolons) separate lines:
    /// `"123 N Main St Apt 4, Miami, FL 33101"`.
    ///
    /// Returns an error only for empty input; anything the parser could not
    /// place is reported in [`ParsedAddress::diagnostics`].
    pub fn parse(&self, input: &str) -> Result<ParsedAddress, ParseError> {
        let cleaned = clean(input);
        if !cleaned
            .text
            .chars()
            .any(|c| c.is_alphanumeric() || c == '-')
        {
            return Err(ParseError::EmptyInput);
        }
        let toks = tokenize(&cleaned, None, 0, &self.lex);
        let lens = LineLengths {
            seg: seg_lengths(&toks, |s| input.get(s.start..s.end)),
            whole: Some(input.trim().chars().count()),
        };
        let tagged = tag_line(&toks, self.lex, lens);
        let mut addr = ParsedAddress::empty(input.to_string(), InputKind::SingleLine, None);
        assemble(&mut addr, &toks, tagged, &self.lex, &|s: &Span| {
            input.get(s.start..s.end)
        });
        Ok(addr)
    }

    /// Parse pre-split fields. Each field is tagged on its own, with the
    /// field as a strong signal (decision 14): the city field is the city.
    /// When only the address lines are given, they are read like a single
    /// line, so a `CITY, ST ZIP` second line still parses.
    pub fn parse_fields(&self, fields: &Fields) -> Result<ParsedAddress, ParseError> {
        let order = [
            FieldName::Line1,
            FieldName::Line2,
            FieldName::City,
            FieldName::State,
            FieldName::Zip,
        ];
        let mut toks: Vec<Token> = Vec::new();
        let mut ranges = Vec::new();
        let mut seg = 0;
        for f in order {
            let start = toks.len();
            if let Some(v) = fields.get(f) {
                let t = tokenize(&clean(v), Some(f), seg, &self.lex);
                seg = t.last().map_or(seg, |t| t.seg) + 1;
                toks.extend(t);
            }
            ranges.push((f, start..toks.len()));
        }
        if toks.is_empty() {
            return Err(ParseError::EmptyInput);
        }
        let source = |s: &Span| s.field.and_then(|f| fields.get(f)?.get(s.start..s.end));
        let lens = LineLengths {
            seg: seg_lengths(&toks, source),
            whole: None,
        };
        let tagged = tag_fields(&toks, &ranges, self.lex, lens);
        let mut addr =
            ParsedAddress::empty(fields.joined(), InputKind::Fields, Some(fields.clone()));
        assemble(&mut addr, &toks, tagged, &self.lex, &source);
        Ok(addr)
    }

    /// Parse many single-line addresses. Each element succeeds or fails
    /// independently. With the `rayon` feature, inputs are parsed in
    /// parallel; results keep the input order.
    pub fn parse_many(&self, inputs: &[&str]) -> Vec<Result<ParsedAddress, ParseError>> {
        #[cfg(feature = "rayon")]
        {
            use rayon::prelude::*;
            inputs.par_iter().map(|s| self.parse(s)).collect()
        }
        #[cfg(not(feature = "rayon"))]
        {
            inputs.iter().map(|s| self.parse(s)).collect()
        }
    }
}

/// Character length of each segment, from its first token to its last.
fn seg_lengths<'a>(toks: &[Token], text: impl Fn(&Span) -> Option<&'a str>) -> Vec<usize> {
    let n = toks.last().map_or(0, |t| t.seg + 1);
    let mut lens = vec![0; n];
    let mut k = 0;
    while k < toks.len() {
        let seg = toks[k].seg;
        let mut e = k;
        while e + 1 < toks.len() && toks[e + 1].seg == seg {
            e += 1;
        }
        let span = Span {
            field: toks[k].span.field,
            start: toks[k].span.start,
            end: toks[e].span.end,
        };
        if toks[e].span.field == span.field {
            if let (Some(s), Some(l)) = (text(&span), lens.get_mut(seg)) {
                *l = s.chars().count();
            }
        }
        k = e + 1;
    }
    lens
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::types::{Component, Warning};

    #[test]
    fn empty_input_is_an_error() {
        assert_eq!(Parser::new().parse("   "), Err(ParseError::EmptyInput));
        assert_eq!(Parser::new().parse(" , "), Err(ParseError::EmptyInput));
        assert_eq!(
            Parser::new().parse_fields(&Fields::default()),
            Err(ParseError::EmptyInput)
        );
    }

    #[test]
    fn parses_a_full_address() {
        let a = Parser::new()
            .parse("123 North Main Street Apt 4, Miami, FL 33101-1234")
            .unwrap();
        assert_eq!(a.street_number.as_deref(), Some("123"));
        assert_eq!(a.street_dir_prefix.map(|d| d.abbr()), Some("N"));
        assert_eq!(a.street_name.as_deref(), Some("MAIN"));
        assert_eq!(a.street_suffix.map(|s| s.abbr()), Some("ST"));
        assert_eq!(a.unit_type.map(|u| u.abbr()), Some("APT"));
        assert_eq!(a.unit_number.as_deref(), Some("4"));
        assert_eq!(a.city.as_deref(), Some("MIAMI"));
        assert_eq!(a.state.map(|s| s.code()), Some("FL"));
        assert_eq!(a.zip5.as_deref(), Some("33101"));
        assert_eq!(a.zip4.as_deref(), Some("1234"));
        assert!(a.diagnostics.warnings.is_empty(), "{:?}", a.diagnostics);
    }

    #[test]
    fn spans_point_into_the_original() {
        let a = Parser::new().parse("123 n. Main St., Miami FL").unwrap();
        let name = a.source.spans_of(Component::StreetName).next().unwrap();
        assert_eq!(a.source.text(name), Some("Main"));
        let sfx = a.source.spans_of(Component::StreetSuffix).next().unwrap();
        assert_eq!(a.source.text(sfx), Some("St."));
        let dir = a
            .source
            .spans_of(Component::StreetDirPrefix)
            .next()
            .unwrap();
        assert_eq!(a.source.text(dir), Some("n."));
    }

    #[test]
    fn fields_are_parsed_separately() {
        let f = Fields {
            line1: Some("1 Main St".into()),
            line2: Some("Ste 200".into()),
            city: Some("Boca Raton, FL".into()),
            zip: Some("33431".into()),
            ..Default::default()
        };
        let a = Parser::new().parse_fields(&f).unwrap();
        assert_eq!(a.source.kind, InputKind::Fields);
        assert_eq!(
            a.source.original,
            "1 Main St, Ste 200, Boca Raton, FL, 33431"
        );
        assert_eq!(a.street_name.as_deref(), Some("MAIN"));
        assert_eq!(a.unit_type.map(|u| u.abbr()), Some("STE"));
        assert_eq!(a.unit_number.as_deref(), Some("200"));
        assert_eq!(a.city.as_deref(), Some("BOCA RATON"));
        assert_eq!(a.state.map(|s| s.code()), Some("FL"));
        let city = a.source.spans_of(Component::City).next().unwrap();
        assert_eq!(city.field, Some(FieldName::City));
        assert_eq!(a.source.text(city), Some("Boca Raton"));
        assert!(a.diagnostics.warnings.is_empty(), "{:?}", a.diagnostics);
    }

    #[test]
    fn unknown_state_field() {
        let f = Fields {
            line1: Some("1 Main St".into()),
            city: Some("Springfield".into()),
            state: Some("Zz".into()),
            zip: Some("1234".into()),
            ..Default::default()
        };
        let a = Parser::new().parse_fields(&f).unwrap();
        assert!(a.diagnostics.has(&Warning::UnknownState));
        assert!(a.diagnostics.has(&Warning::InvalidZip));
        assert!(!a.diagnostics.has(&Warning::MissingState));
        assert_eq!(a.diagnostics.unparsed.as_deref(), Some("Zz 1234"));
    }

    /// Random token soup never panics, and every span indexes the input.
    #[test]
    fn never_panics_and_spans_are_valid() {
        let words = [
            "1",
            "23",
            "#",
            "-",
            "(",
            ")",
            ",",
            ".",
            "/",
            "A",
            "N",
            "ST",
            "APT",
            "PO",
            "BOX",
            "RR",
            "HC",
            "FL",
            "33101",
            "APO",
            "AE",
            "UNIT",
            "HWY",
            "I",
            "10",
            "É",
            "\u{a0}",
            "½",
            "—",
            "GENERAL",
            "DELIVERY",
            "NO",
            "SITUS",
            "N/A",
            "CT",
            "BLDG",
            "0",
            "12A",
            "1/2",
            "5210/5212",
            "409742ND",
            "CTAPT",
            "TAMPA33616",
            "PSC",
            "日本",
            ";",
            "'",
        ];
        let p = Parser::new();
        let mut seed: u64 = 0x2545_F491_4F6C_DD1D;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for _ in 0..20_000 {
            let mut s = String::new();
            for _ in 0..next() % 12 {
                s.push_str(words[(next() % words.len() as u64) as usize]);
                if next() % 2 == 0 {
                    s.push(' ');
                }
            }
            let fields = Fields {
                line1: Some(s.clone()),
                city: Some(s.chars().rev().collect()),
                ..Default::default()
            };
            for a in [p.parse(&s), p.parse_fields(&fields)].into_iter().flatten() {
                for c in &a.source.spans {
                    assert!(a.source.text(&c.span).is_some(), "{s:?}: {c:?}");
                }
            }
        }
    }

    #[test]
    fn parse_many_keeps_order() {
        let r = Parser::new().parse_many(&["1 A ST", "", "2 B AVE"]);
        assert_eq!(r[0].as_ref().unwrap().street_number.as_deref(), Some("1"));
        assert_eq!(r[1], Err(ParseError::EmptyInput));
        assert_eq!(r[2].as_ref().unwrap().street_number.as_deref(), Some("2"));
    }
}
