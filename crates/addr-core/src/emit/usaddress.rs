//! usaddress-compatible token labels (design decision 11, section 8): each
//! whitespace-delimited token of the input with the label the Python
//! `usaddress` library uses for it (`AddressNumber`, `StreetName`, ...). For
//! migration and for comparing the two parsers on usaddress's labeled data
//! (`tools/compare/`). Built from the component spans; it never re-parses.
//!
//! This formats a parse; it does not check that the address exists or is
//! deliverable.

use crate::lexicon::{Lexicons, Special};
use crate::types::{Component, FieldName, InputKind, ParsedAddress, Span};

/// One input token and its usaddress label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsaddressTag {
    /// The token as written, punctuation included (`Road,`).
    pub text: String,
    /// Where the token is: in [`crate::SourceInfo::original`] for single-line
    /// input, in its field for [`crate::Fields`] input.
    pub span: Span,
    /// The usaddress label, or `None` for a token the parse did not place
    /// (unparsed text, or a skipped copy of the city or ZIP).
    pub label: Option<&'static str>,
}

const FIELD_ORDER: [FieldName; 5] = [
    FieldName::Line1,
    FieldName::Line2,
    FieldName::City,
    FieldName::State,
    FieldName::Zip,
];

impl ParsedAddress {
    /// The input's tokens with usaddress labels, in input order (fields in
    /// `line1, line2, city, state, zip` order).
    ///
    /// Mapping: street number `AddressNumber`; directionals
    /// `StreetNamePreDirectional` / `StreetNamePostDirectional`; name
    /// `StreetName`; suffix `StreetNamePostType`; a highway's state and
    /// prefix `StreetNamePreType` and its route `StreetName`; units
    /// `OccupancyType` / `OccupancyIdentifier`; PO Box `USPSBoxType` /
    /// `USPSBoxID`; rural, highway contract and military box lines
    /// `USPSBoxGroupType` / `USPSBoxGroupID` / `USPSBoxType` / `USPSBoxID`;
    /// general delivery `USPSBoxType`; city and APO/FPO/DPO `PlaceName`;
    /// state `StateName`; ZIP `ZipCode`; attention line `Recipient`. A `#`
    /// takes the label of the value after it, as in usaddress's own data.
    pub fn to_usaddress_tags(&self) -> Vec<UsaddressTag> {
        let mut tags = Vec::new();
        match (self.source.kind, &self.source.fields) {
            (InputKind::Fields, Some(fields)) => {
                for f in FIELD_ORDER {
                    if let Some(text) = fields.get(f) {
                        tokens(text, Some(f), &mut tags);
                    }
                }
            }
            _ => tokens(&self.source.original, None, &mut tags),
        }
        for t in &mut tags {
            t.label = self.label_at(&t.span);
        }
        fix_unlabeled(&mut tags);
        tags
    }

    /// The label of the first component span overlapping `tok`.
    fn label_at(&self, tok: &Span) -> Option<&'static str> {
        let overlaps = |s: &Span| s.field == tok.field && s.start < tok.end && tok.start < s.end;
        let hit = self
            .source
            .spans
            .iter()
            .filter(|c| overlaps(&c.span))
            .min_by_key(|c| c.span.start)?;
        // Position of this span among the spans of its component, for the
        // components that cover several kinds of token.
        let mut same: Vec<&Span> = self.source.spans_of(hit.component).collect();
        same.sort_by_key(|s| (s.field.map(field_index), s.start));
        let i = same.iter().position(|s| **s == hit.span).unwrap_or(0);
        let n = same.len();
        let box_line = |i: usize| match i {
            0 => "USPSBoxGroupType",
            1 => "USPSBoxGroupID",
            _ => "USPSBoxID",
        };
        Some(match hit.component {
            Component::StreetNumber => "AddressNumber",
            Component::StreetDirPrefix => "StreetNamePreDirectional",
            Component::StreetName => "StreetName",
            Component::StreetSuffix => "StreetNamePostType",
            Component::StreetDirSuffix => "StreetNamePostDirectional",
            Component::Highway if i + 1 == n => "StreetName",
            Component::Highway => "StreetNamePreType",
            Component::UnitType | Component::SecondUnitType => "OccupancyType",
            Component::UnitNumber | Component::SecondUnitNumber => "OccupancyIdentifier",
            Component::PoBox if i + 1 == n => "USPSBoxID",
            Component::PoBox => "USPSBoxType",
            Component::RuralRoute | Component::Military => box_line(i),
            Component::GeneralDelivery => "USPSBoxType",
            Component::MilitaryPostOffice | Component::City => "PlaceName",
            Component::State => "StateName",
            Component::Zip5 | Component::Zip4 => "ZipCode",
            Component::Attention => "Recipient",
            Component::Unparsed => return None,
        })
    }
}

fn field_index(f: FieldName) -> usize {
    FIELD_ORDER.iter().position(|&g| g == f).unwrap_or(0)
}

/// Whitespace-delimited tokens of `text`, with byte offsets.
fn tokens(text: &str, field: Option<FieldName>, out: &mut Vec<UsaddressTag>) {
    let mut start = None;
    for (i, c) in text.char_indices().chain([(text.len(), ' ')]) {
        match (c.is_whitespace(), start) {
            (false, None) => start = Some(i),
            (true, Some(s)) => {
                out.push(UsaddressTag {
                    text: text[s..i].to_string(),
                    span: Span {
                        field,
                        start: s,
                        end: i,
                    },
                    label: None,
                });
                start = None;
            }
            _ => {}
        }
    }
}

/// Tokens the parser consumed without a value: a `#` takes the next
/// token's label (`# 234`, `RR # 1`), and the BOX word of a route or
/// military line is `USPSBoxType`.
fn fix_unlabeled(tags: &mut [UsaddressTag]) {
    let lex = Lexicons::builtin();
    for k in (0..tags.len()).rev() {
        if tags[k].label.is_some() {
            continue;
        }
        let next = tags.get(k + 1).and_then(|t| t.label);
        let word = tags[k]
            .text
            .trim_matches(|c: char| !c.is_alphanumeric() && c != '#');
        if word == "#" {
            tags[k].label = next;
        } else if next == Some("USPSBoxID")
            && lex.special(word).is_some_and(|l| l.value == Special::Box)
        {
            tags[k].label = Some("USPSBoxType");
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::{Fields, Parser};

    fn tags(input: &str) -> Vec<(String, Option<&'static str>)> {
        let a = Parser::new().parse(input).unwrap();
        a.to_usaddress_tags()
            .into_iter()
            .map(|t| (t.text, t.label))
            .collect()
    }

    fn labels(input: &str) -> Vec<&'static str> {
        tags(input)
            .into_iter()
            .map(|(_, l)| l.unwrap_or("-"))
            .collect()
    }

    #[test]
    fn street_address() {
        assert_eq!(
            labels("123 N. Main Street, Apt 4, Santa Fe NM 87505"),
            [
                "AddressNumber",
                "StreetNamePreDirectional",
                "StreetName",
                "StreetNamePostType",
                "OccupancyType",
                "OccupancyIdentifier",
                "PlaceName",
                "PlaceName",
                "StateName",
                "ZipCode"
            ]
        );
        assert_eq!(tags("123 Main St, Miami FL 33101")[2].0, "St,");
    }

    #[test]
    fn pound_takes_the_next_label() {
        assert_eq!(
            labels("1000 Cordova Place # 234, Santa Fe NM 87505")[3..5],
            ["OccupancyIdentifier", "OccupancyIdentifier"]
        );
        assert_eq!(
            labels("rr # 1 box 54 loami il 62661")[..5],
            [
                "USPSBoxGroupType",
                "USPSBoxGroupID",
                "USPSBoxGroupID",
                "USPSBoxType",
                "USPSBoxID"
            ]
        );
    }

    #[test]
    fn highway_and_po_box() {
        assert_eq!(
            labels("1234 W US Hwy 50"),
            [
                "AddressNumber",
                "StreetNamePreDirectional",
                "StreetNamePreType",
                "StreetNamePreType",
                "StreetName"
            ]
        );
        assert_eq!(
            labels("PO Box 12")[..],
            ["USPSBoxType", "USPSBoxType", "USPSBoxID"]
        );
    }

    #[test]
    fn unparsed_tokens_have_no_label() {
        assert_eq!(labels("409742ND SQ")[..], ["-", "-"]);
    }

    #[test]
    fn fields_are_tokenized_one_by_one() {
        let a = Parser::new()
            .parse_fields(&Fields {
                line1: Some("1 Main St".into()),
                line2: Some("ATTN LEGAL".into()),
                city: Some("Tampa".into()),
                state: Some("FL".into()),
                zip: Some("33601".into()),
            })
            .unwrap();
        let t = a.to_usaddress_tags();
        assert_eq!(t.len(), 8);
        assert_eq!(t[3].label, Some("Recipient"));
        assert_eq!(t[7].span.field, Some(crate::FieldName::Zip));
        assert_eq!((t[7].span.start, t[7].span.end), (0, 5));
    }
}
