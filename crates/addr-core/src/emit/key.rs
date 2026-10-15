//! Canonical match key (format `k1`). The spec, with the reasoning behind
//! each rule, is `docs/key-format.md`; changing what any input produces needs
//! a new version prefix, never an in-place edit.

use std::fmt::Write;

use crate::types::{ParsedAddress, Warning};

/// Options for [`ParsedAddress::key`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KeyOptions {
    /// Add the unit numbers to the key. `false` (the default) gives a
    /// property-level key, so every unit in a building shares one key.
    pub include_unit: bool,
}

impl KeyOptions {
    /// Property-level key: units ignored.
    pub const fn property() -> Self {
        KeyOptions {
            include_unit: false,
        }
    }

    /// Unit-level key: the unit numbers are part of the key.
    pub const fn unit() -> Self {
        KeyOptions { include_unit: true }
    }
}

impl ParsedAddress {
    /// The canonical match key, or `None` when the address has nothing to key
    /// on: no street, PO box, route, military line or general delivery, or a
    /// placeholder such as "NO SITUS". Format: `docs/key-format.md`.
    ///
    /// Two inputs that differ only in spelling (`North`/`N`, `Street`/`ST`,
    /// `Apt 4`/`#4`, case, punctuation) give the same key. The key says
    /// nothing about whether the address exists or is deliverable.
    pub fn key(&self, options: KeyOptions) -> Option<String> {
        if self.diagnostics.has(&Warning::Placeholder) {
            return None;
        }

        let mut segs: Vec<String> = Vec::new();
        let form = if let Some(po) = &self.po_box {
            segs.push(esc(po));
            'P'
        } else if let Some(r) = &self.rural_route {
            segs.extend([
                r.kind.abbr().to_string(),
                esc(&r.route),
                r.box_number.as_deref().map(esc).unwrap_or_default(),
            ]);
            'R'
        } else if let Some(m) = &self.military {
            segs.extend([
                m.kind.abbr().to_string(),
                esc(&m.number),
                m.box_number.as_deref().map(esc).unwrap_or_default(),
                self.military_post_office
                    .map(|p| p.abbr().to_string())
                    .unwrap_or_default(),
            ]);
            'M'
        } else if self.general_delivery {
            'G'
        } else if self.street_name.is_some() || self.highway.is_some() {
            let name = match &self.highway {
                Some(h) => h.name(),
                None => self.street_name.clone().unwrap_or_default(),
            };
            segs.extend([
                self.street_number.as_deref().map(esc).unwrap_or_default(),
                self.street_dir_prefix
                    .map(|d| d.abbr().to_string())
                    .unwrap_or_default(),
                esc(&name),
                self.street_suffix
                    .map(|s| s.abbr().to_string())
                    .unwrap_or_default(),
                self.street_dir_suffix
                    .map(|d| d.abbr().to_string())
                    .unwrap_or_default(),
            ]);
            'S'
        } else {
            return None;
        };

        let state = self.state.map(|s| s.code().to_string());
        let zip5 = self
            .zip5
            .as_deref()
            .filter(|z| z.len() == 5 && z.bytes().all(|b| b.is_ascii_digit()));
        let located = state.is_some() && zip5.is_some();

        segs.push(state.unwrap_or_default());
        segs.push(zip5.unwrap_or_default().to_string());
        if options.include_unit && form == 'S' {
            segs.push(self.unit_number.as_deref().map(esc).unwrap_or_default());
            segs.push(
                self.second_unit_number
                    .as_deref()
                    .map(esc)
                    .unwrap_or_default(),
            );
        }

        let mut key = String::from(if located { "k1:" } else { "k1n:" });
        key.push(form);
        for s in &segs {
            key.push('|');
            key.push_str(s);
        }
        Some(key)
    }
}

/// Escape one segment: uppercase, drop periods and apostrophes (Pub 28 uses
/// minimal punctuation), collapse whitespace, then percent-encode every byte
/// outside `A-Z 0-9 space / -` so a segment can never contain `|` or `%`
/// unescaped.
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut pending_space = false;
    for c in s.chars() {
        if matches!(c, '.' | '\'' | '\u{2019}') {
            continue;
        }
        if c.is_whitespace() {
            pending_space = !out.is_empty();
            continue;
        }
        if pending_space {
            out.push(' ');
            pending_space = false;
        }
        for u in c.to_uppercase() {
            if u.is_ascii_uppercase() || u.is_ascii_digit() || matches!(u, '/' | '-') {
                out.push(u);
            } else {
                let mut buf = [0u8; 4];
                for b in u.encode_utf8(&mut buf).bytes() {
                    let _ = write!(out, "%{b:02X}");
                }
            }
        }
    }
    out
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Fields, Parser};

    fn key(input: &str, o: KeyOptions) -> Option<String> {
        Parser::new().parse(input).unwrap().key(o)
    }

    fn pk(input: &str) -> String {
        key(input, KeyOptions::default()).unwrap()
    }

    #[test]
    fn spelling_variants_share_a_key() {
        let a = pk("123 N. Main Street, Apt 4, Miami FL 33101");
        assert_eq!(a, "k1:S|123|N|MAIN|ST||FL|33101");
        assert_eq!(a, pk("123 north main st #4, miami, florida 33101-1234"));
        assert_eq!(a, pk("123 NORTH MAIN ST, MIAMI, FL 33101"));
    }

    #[test]
    fn default_is_property_level() {
        assert_eq!(KeyOptions::default(), KeyOptions::property());
    }

    #[test]
    fn unit_level_key() {
        let u = KeyOptions::unit();
        let a = key("123 N Main St Apt 4, Miami FL 33101", u).unwrap();
        assert_eq!(a, "k1:S|123|N|MAIN|ST||FL|33101|4|");
        assert_eq!(
            a,
            key("123 north main street #4, Miami FL 33101", u).unwrap()
        );
        assert_ne!(a, key("123 N Main St Apt 5, Miami FL 33101", u).unwrap());
        // The property key is the unit key minus its two unit segments.
        assert!(a.starts_with(&pk("123 N Main St Apt 4, Miami FL 33101")));
        // No unit is not the same as a unit, at unit level.
        assert_ne!(a, key("123 N Main St, Miami FL 33101", u).unwrap());
    }

    #[test]
    fn city_and_zip4_are_not_in_the_key() {
        assert_eq!(
            pk("1 Main St, Miami FL 33101"),
            pk("1 Main St, Miami Beach FL 33101-0001")
        );
    }

    #[test]
    fn missing_zip_or_state_gets_the_n_prefix() {
        assert_eq!(pk("123 Main St, Miami FL"), "k1n:S|123||MAIN|ST||FL|");
        assert_eq!(pk("123 Main St"), "k1n:S|123||MAIN|ST|||");
        assert!(pk("123 Main St, Miami 33101").starts_with("k1n:"));
        assert!(pk("123 Main St, Miami FL 33101").starts_with("k1:"));
    }

    #[test]
    fn special_forms_never_equal_a_street_key() {
        assert_eq!(pk("PO Box 12, Tampa FL 33601"), "k1:P|12|FL|33601");
        assert_eq!(pk("P.O. BOX 12, Tampa FL 33601"), "k1:P|12|FL|33601");
        assert_eq!(pk("RR 2 Box 152, Ocala FL 34470"), "k1:R|RR|2|152|FL|34470");
        assert_eq!(
            pk("PSC 1650 Box 10, APO AE 09001"),
            "k1:M|PSC|1650|10|APO|AE|09001"
        );
        assert_eq!(pk("General Delivery, Tampa FL 33601"), "k1:G|FL|33601");
        assert_eq!(
            pk("100 County Road 23, Ocala FL 34470"),
            "k1:S|100||COUNTY ROAD 23|||FL|34470"
        );
    }

    #[test]
    fn units_do_not_apply_to_po_boxes() {
        let a = Parser::new().parse("PO Box 12, Tampa FL 33601").unwrap();
        assert_eq!(a.key(KeyOptions::unit()), a.key(KeyOptions::property()));
    }

    #[test]
    fn nothing_to_key_on() {
        assert_eq!(key("Tampa FL 33601", KeyOptions::default()), None);
        assert_eq!(key("NO SITUS", KeyOptions::default()), None);
    }

    #[test]
    fn escaping() {
        assert_eq!(esc("O'Brien  St."), "OBRIEN ST");
        assert_eq!(esc("A|B%C"), "A%7CB%25C");
        assert_eq!(esc("1/2"), "1/2");
    }

    #[test]
    fn fields_input_matches_single_line() {
        let f = Fields {
            line1: Some("123 N Main St".into()),
            city: Some("Miami".into()),
            state: Some("FL".into()),
            zip: Some("33101".into()),
            ..Default::default()
        };
        let a = Parser::new().parse_fields(&f).unwrap();
        assert_eq!(
            a.key(KeyOptions::default()).unwrap(),
            pk("123 N Main St, Miami FL 33101")
        );
    }
}
