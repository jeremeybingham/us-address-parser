//! URL-safe slug. Display-oriented, not a key: two different addresses can
//! share a slug and one address can change slug when parsing improves. Use
//! [`ParsedAddress::key`] for dedupe and joins. Spec: `docs/key-format.md`.

use crate::types::{ParsedAddress, Warning};

impl ParsedAddress {
    /// A lowercase, hyphenated, ASCII slug such as
    /// `123-n-main-st-apt-4-miami-fl-33101`, or `None` when there is nothing
    /// to name (no address lines, or a placeholder such as "NO SITUS").
    ///
    /// Built from the Pub 28 style output ([`Self::to_usps`]): delivery line
    /// with its unit, city, state and ZIP5. ZIP+4 is left out. A dual
    /// address (street and PO Box) uses the street and its units, like the
    /// key and the RESO output, not the PO Box the USPS output writes. The
    /// slug says nothing about whether the address exists or is deliverable.
    pub fn slug(&self) -> Option<String> {
        if self.diagnostics.has(&Warning::Placeholder) {
            return None;
        }
        let u = self.to_usps();
        let line1 = if self.is_dual() {
            self.street_line()
                .map(|s| format!("{s} {}", self.units_text()))
        } else {
            u.line1
        };
        let parts = [line1, u.city, u.state, u.zip.map(|z| z[..5].to_string())];
        let text = parts.into_iter().flatten().collect::<Vec<_>>().join(" ");
        let slug = slugify(&text);
        (!slug.is_empty()).then_some(slug)
    }
}

/// Lowercase ASCII letters and digits, with every other run of characters
/// collapsed to one hyphen. Apostrophes are dropped (`O'BRIEN` -> `obrien`)
/// and `#` reads as `unit` (Pub 28 C2 writes `# 4` when the designator is
/// unknown). No leading or trailing hyphen.
fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut pending = false;
    for c in s.chars() {
        let piece: &str = match c {
            '\'' | '\u{2019}' => continue,
            '#' => "unit",
            _ => "",
        };
        if c.is_ascii_alphanumeric() || !piece.is_empty() {
            if pending && !out.is_empty() {
                out.push('-');
            }
            pending = false;
            if piece.is_empty() {
                out.push(c.to_ascii_lowercase());
            } else {
                out.push_str(piece);
            }
        } else {
            pending = true;
        }
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::{Fields, Parser};

    fn slug(input: &str) -> Option<String> {
        Parser::new().parse(input).unwrap().slug()
    }

    #[test]
    fn spelling_variants_give_one_slug() {
        let want = Some("123-n-main-st-apt-4-miami-fl-33101".to_string());
        assert_eq!(slug("123 N. Main Street, Apt 4, Miami FL 33101"), want);
        assert_eq!(
            slug("123 north main st apt 4, miami, florida 33101-1234"),
            want
        );
    }

    #[test]
    fn hash_unit_reads_as_unit() {
        assert_eq!(
            slug("123 Main St #4, Miami FL 33101").as_deref(),
            Some("123-main-st-unit-4-miami-fl-33101")
        );
    }

    #[test]
    fn punctuation_collapses_and_apostrophes_drop() {
        assert_eq!(
            slug("9 O'Brien Rd 1/2, St. Petersburg FL 33701").as_deref(),
            slug("9 OBrien Rd 1/2, St Petersburg FL 33701").as_deref()
        );
        let s = slug("39.2 County Road, Tampa FL 33601").unwrap();
        assert!(!s.starts_with('-') && !s.ends_with('-') && !s.contains("--"));
        assert!(s
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'));
    }

    #[test]
    fn partial_and_special_forms() {
        assert_eq!(slug("123 Main St").as_deref(), Some("123-main-st"));
        assert_eq!(
            slug("PO Box 12, Tampa FL 33601").as_deref(),
            Some("po-box-12-tampa-fl-33601")
        );
    }

    #[test]
    fn dual_address_follows_the_street() {
        assert_eq!(
            slug("265 Aspen Dr Unit 2 PO Box 2504, Jackson WY 83001").as_deref(),
            Some("265-aspen-dr-unit-2-jackson-wy-83001")
        );
        assert_eq!(
            slug("1679 E Orangethorpe Ave PO Box 365, Atwood CA 92811").as_deref(),
            Some("1679-e-orangethorpe-ave-atwood-ca-92811")
        );
    }

    #[test]
    fn no_slug_for_placeholder_or_nothing() {
        assert_eq!(slug("NO SITUS"), None);
        let a = Parser::new().parse_fields(&Fields::default());
        assert!(a.map_or(true, |a| a.slug().is_none()));
    }
}
