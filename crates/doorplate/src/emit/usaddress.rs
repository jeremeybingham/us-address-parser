//! usaddress-compatible token labels (design decision 11): each
//! whitespace-delimited token of the input with the label the Python
//! `usaddress` library uses for it (`AddressNumber`, `StreetName`, ...). For
//! migration and for comparing the two parsers on usaddress's labeled data
//! (`tools/compare/`). Built from the component spans; it never re-parses.
//!
//! This formats a parse; it does not check that the address exists or is
//! deliverable.

use crate::lexicon::{Lexicons, Special};
use crate::types::{Component, FieldName, InputKind, ParsedAddress, Span, Warning};

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

/// What kind of address a record is. Finer than usaddress's three values
/// ([`TaggedAddress::usaddress_type`] gives those, for migration).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AddressType {
    /// A street address (a street name or a highway).
    Street,
    /// A PO Box (or drawer, caller, ...) alone.
    PoBox,
    /// A rural route or highway contract route.
    RuralRoute,
    /// A military or Department of State line.
    Military,
    /// General delivery.
    GeneralDelivery,
    /// A street address and a PO Box in one record (Pub 28 section 215).
    Dual,
    /// None of the above: nothing to deliver to, or a placeholder.
    Unknown,
}

impl AddressType {
    /// Stable name: `"street"`, `"po_box"`, `"rural_route"`, `"military"`,
    /// `"general_delivery"`, `"dual"`, `"unknown"`.
    pub const fn code(&self) -> &'static str {
        match self {
            AddressType::Street => "street",
            AddressType::PoBox => "po_box",
            AddressType::RuralRoute => "rural_route",
            AddressType::Military => "military",
            AddressType::GeneralDelivery => "general_delivery",
            AddressType::Dual => "dual",
            AddressType::Unknown => "unknown",
        }
    }
}

/// The parse joined by usaddress label, as `usaddress.tag()` returns it:
/// the tokens that share a label, joined with a space in input order.
///
/// Unlike `usaddress.tag()` it never fails: a label in two separate runs is
/// joined and reported as [`Warning::RepeatedLabel`] (usaddress raises
/// `RepeatedLabelError`). Tokens the parse did not place have no label and
/// are left out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaggedAddress {
    components: Vec<(&'static str, String)>,
    address_type: AddressType,
    repeated: Vec<&'static str>,
}

impl TaggedAddress {
    /// The joined value of one label (`"StreetName"`).
    pub fn get(&self, label: &str) -> Option<&str> {
        self.components
            .iter()
            .find(|(l, _)| *l == label)
            .map(|(_, v)| v.as_str())
    }

    /// Label and value pairs, in the order each label first appears.
    pub fn components(&self) -> &[(&'static str, String)] {
        &self.components
    }

    /// The kind of address.
    pub fn address_type(&self) -> AddressType {
        self.address_type
    }

    /// `usaddress.tag()`'s second value, worked out the way it does: `"Street
    /// Address"` when an `AddressNumber` is present, else `"PO Box"` when a
    /// `USPSBoxID` is, else `"Ambiguous"`. (usaddress's `"Intersection"` is
    /// not modeled.) A dual address is a `"Street Address"` here, as there.
    pub fn usaddress_type(&self) -> &'static str {
        if self.get("AddressNumber").is_some() {
            "Street Address"
        } else if self.get("USPSBoxID").is_some() {
            "PO Box"
        } else {
            "Ambiguous"
        }
    }

    /// [`Warning::RepeatedLabel`] when a label was joined from separate
    /// runs; empty otherwise.
    pub fn warnings(&self) -> Vec<Warning> {
        if self.repeated.is_empty() {
            Vec::new()
        } else {
            vec![Warning::RepeatedLabel]
        }
    }

    /// The labels that were joined from separate runs.
    pub fn repeated_labels(&self) -> &[&'static str] {
        &self.repeated
    }
}

const FIELD_ORDER: [FieldName; 5] = [
    FieldName::Line1,
    FieldName::Line2,
    FieldName::City,
    FieldName::State,
    FieldName::Zip,
];

impl ParsedAddress {
    /// The kind of address: street, PO Box, rural route, military, general
    /// delivery, dual (a street and a PO Box) or unknown.
    pub fn address_type(&self) -> AddressType {
        if self.is_dual() {
            AddressType::Dual
        } else if self.po_box.is_some() {
            AddressType::PoBox
        } else if self.rural_route.is_some() {
            AddressType::RuralRoute
        } else if self.military.is_some() {
            AddressType::Military
        } else if self.general_delivery {
            AddressType::GeneralDelivery
        } else if self.has_street() {
            AddressType::Street
        } else {
            AddressType::Unknown
        }
    }

    /// The parse joined by usaddress label, the counterpart of
    /// `usaddress.tag()` (design.md decision 23): tokens that share a label
    /// are joined with a space in input order, the ends of each value are
    /// stripped of spaces, commas and semicolons, and tokens the parse did
    /// not place are left out. A label in two separate runs is joined and
    /// reported ([`TaggedAddress::repeated_labels`]) instead of failing.
    /// Built from [`Self::to_usaddress_tags`]; it never re-parses.
    pub fn tag(&self) -> TaggedAddress {
        let mut runs: Vec<(&'static str, Vec<String>)> = Vec::new();
        let mut repeated: Vec<&'static str> = Vec::new();
        let mut last: Option<&'static str> = None;
        for t in self.to_usaddress_tags() {
            let Some(label) = t.label else { continue };
            match runs.iter_mut().find(|(l, _)| *l == label) {
                Some((_, toks)) => {
                    if last != Some(label) && !repeated.contains(&label) {
                        repeated.push(label);
                    }
                    toks.push(t.text);
                }
                None => runs.push((label, vec![t.text])),
            }
            last = Some(label);
        }
        let components = runs
            .into_iter()
            .filter_map(|(label, toks)| {
                let value = toks.join(" ");
                let value = value.trim_matches([' ', ',', ';']);
                (!value.is_empty()).then(|| (label, value.to_string()))
            })
            .collect();
        TaggedAddress {
            components,
            address_type: self.address_type(),
            repeated,
        }
    }

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
    /// state `StateName`; ZIP `ZipCode`; attention line `Recipient`; building
    /// `BuildingName`. A `#`
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
        // A route or military line: the kind word (RR, HC, PSC, ...), the
        // number after it, then the box. Found by the kind word, not by
        // position, so `BOX 2877 PSC 103` is labeled right too.
        let lex = Lexicons::builtin();
        let is_kind = |s: &Span| {
            self.source.text(s).is_some_and(|t| {
                lex.military(t).is_some()
                    || lex.special(t).is_some_and(|l| {
                        matches!(l.value, Special::RuralRoute | Special::HighwayContract)
                    })
            })
        };
        let box_line = || match same.iter().position(|s| is_kind(s)) {
            Some(k) if i == k => "USPSBoxGroupType",
            Some(k) if i == k + 1 => "USPSBoxGroupID",
            Some(_) => "USPSBoxID",
            None => "USPSBoxID",
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
            Component::RuralRoute | Component::Military => box_line(),
            Component::GeneralDelivery => "USPSBoxType",
            Component::MilitaryPostOffice | Component::City => "PlaceName",
            Component::State => "StateName",
            Component::Zip5 | Component::Zip4 => "ZipCode",
            Component::Attention => "Recipient",
            Component::Building => "BuildingName",
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
    fn tag_joins_tokens_by_label() {
        let t = Parser::new()
            .parse("123 Martin Luther King Jr Blvd, Apt 4, Santa Fe NM 87505")
            .unwrap()
            .tag();
        assert_eq!(t.get("AddressNumber"), Some("123"));
        // Words that share a label are joined with a space, in order; the
        // comma after a token is not part of the value.
        assert_eq!(t.get("PlaceName"), Some("Santa Fe"));
        assert_eq!(t.get("StreetNamePostType"), Some("Blvd"));
        assert_eq!(t.get("StreetName"), Some("Martin Luther King Jr"));
        // The first appearance of each label sets the order.
        let labels: Vec<_> = t.components().iter().map(|(l, _)| *l).collect();
        assert_eq!(labels[0], "AddressNumber");
        assert_eq!(labels.last(), Some(&"ZipCode"));
        assert!(t.warnings().is_empty() && t.repeated_labels().is_empty());
        assert_eq!(t.usaddress_type(), "Street Address");
    }

    #[test]
    fn tag_reports_a_repeated_label_instead_of_failing() {
        let t = Parser::new()
            .parse("123 Main St Apt 4 Ste 5, Miami FL 33101")
            .unwrap()
            .tag();
        assert_eq!(t.get("OccupancyType"), Some("Apt Ste"));
        assert_eq!(t.get("OccupancyIdentifier"), Some("4 5"));
        assert_eq!(
            t.repeated_labels(),
            ["OccupancyType", "OccupancyIdentifier"]
        );
        assert_eq!(t.warnings(), [crate::Warning::RepeatedLabel]);
    }

    #[test]
    fn address_types() {
        let ty = |s: &str| Parser::new().parse(s).unwrap().address_type();
        use crate::AddressType::*;
        assert_eq!(ty("123 Main St, Miami FL 33101"), Street);
        assert_eq!(ty("US Hwy 50, Carson City NV"), Street);
        assert_eq!(ty("PO Box 12, Tampa FL 33601"), PoBox);
        assert_eq!(ty("RR 2 Box 15, Loami IL 62661"), RuralRoute);
        assert_eq!(ty("PSC 1234 Box 56, APO AE 09012"), Military);
        assert_eq!(ty("General Delivery, Tampa FL 33601"), GeneralDelivery);
        assert_eq!(ty("9418 CUBA RD, PO BOX 100, HAYES VA 23072"), Dual);
        assert_eq!(ty("NO SITUS"), Unknown);
        assert_eq!(ty("Tampa FL 33601"), Unknown);
    }

    #[test]
    fn usaddress_type_follows_usaddress_rules() {
        let t = |s: &str| Parser::new().parse(s).unwrap().tag().usaddress_type();
        assert_eq!(t("123 Main St, Miami FL 33101"), "Street Address");
        assert_eq!(t("PO Box 12, Tampa FL 33601"), "PO Box");
        // A rural route has a USPSBoxID; general delivery has none.
        assert_eq!(t("RR 2 Box 15, Loami IL 62661"), "PO Box");
        assert_eq!(t("General Delivery, Tampa FL 33601"), "Ambiguous");
        // AddressNumber comes first, so a dual address is a Street Address.
        assert_eq!(
            t("9418 CUBA RD, PO BOX 100, HAYES VA 23072"),
            "Street Address"
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
