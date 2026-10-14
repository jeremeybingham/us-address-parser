//! USPS Publication 28 style output. Rules and their sources (or, where not
//! yet checked against the Pub 28 text, their "unverified" status) are in
//! `docs/usps-pub28-notes.md`, section "Emitter rules".
//!
//! This formats a parse; it does not check that the address exists or is
//! deliverable.

use crate::types::{ParsedAddress, UnitType};

/// Longest delivery line before the unit moves to `line2`. Unverified
/// against Pub 28; see the notes.
const MAX_LINE: usize = 40;

/// A Pub 28 style address: uppercase, no punctuation except the hyphen.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UspsAddress {
    /// Delivery line: street, PO Box, route, military line or general
    /// delivery, with the unit on the same line when it fits.
    pub line1: Option<String>,
    /// The unit(s), only when they do not fit on `line1`.
    pub line2: Option<String>,
    /// City, or the military post office (`APO`, `FPO`, `DPO`).
    pub city: Option<String>,
    /// Two-letter state code.
    pub state: Option<String>,
    /// `NNNNN` or `NNNNN-NNNN`.
    pub zip: Option<String>,
}

impl UspsAddress {
    /// `CITY ST ZIP`, with the missing parts left out.
    pub fn last_line(&self) -> String {
        join([&self.city, &self.state, &self.zip])
    }
}

impl ParsedAddress {
    /// The address in USPS Pub 28 style. Components the parse did not find
    /// are left out; [`Diagnostics`](crate::Diagnostics) says what was
    /// missing or unparsed.
    pub fn to_usps(&self) -> UspsAddress {
        let units = join([
            &unit_text(self.unit_type, self.unit_number.as_deref()),
            &unit_text(self.second_unit_type, self.second_unit_number.as_deref()),
        ]);
        let primary = self.primary_line();
        let (line1, line2) = match (primary, units.is_empty()) {
            (p, true) => (p, None),
            (None, false) => (Some(units), None),
            (Some(p), false) if p.len() + 1 + units.len() <= MAX_LINE => {
                (Some(format!("{p} {units}")), None)
            }
            (Some(p), false) => (Some(p), Some(units)),
        };

        let zip = self.zip5.as_ref().map(|z5| match &self.zip4 {
            Some(z4) => format!("{z5}-{z4}"),
            None => z5.clone(),
        });
        UspsAddress {
            line1,
            line2,
            city: self
                .military_post_office
                .map(|p| p.abbr().to_string())
                .or_else(|| self.city.as_deref().map(plain))
                .filter(|c| !c.is_empty()),
            state: self.state.map(|s| s.code().to_string()),
            zip,
        }
    }

    fn primary_line(&self) -> Option<String> {
        let line = if let Some(po) = &self.po_box {
            format!("PO BOX {}", plain(po))
        } else if let Some(r) = &self.rural_route {
            let mut s = format!("{} {}", r.kind.abbr(), plain(&r.route));
            if let Some(b) = &r.box_number {
                s = format!("{s} BOX {}", plain(b));
            }
            s
        } else if let Some(m) = &self.military {
            let mut s = format!("{} {}", m.kind.abbr(), plain(&m.number));
            if let Some(b) = &m.box_number {
                s = format!("{s} BOX {}", plain(b));
            }
            s
        } else if self.general_delivery {
            "GENERAL DELIVERY".to_string()
        } else {
            let name = match (&self.highway, &self.street_name) {
                (Some(h), _) => Some(format!("{} {}", h.prefix.standard(), plain(&h.route))),
                (None, Some(n)) => Some(plain(n)),
                (None, None) => None,
            };
            join([
                &self.street_number.as_deref().map(plain),
                &self.street_dir_prefix.map(|d| d.abbr().to_string()),
                &name,
                &self.street_suffix.map(|s| s.abbr().to_string()),
                &self.street_dir_suffix.map(|d| d.abbr().to_string()),
            ])
        };
        (!line.is_empty()).then_some(line)
    }
}

/// `APT 4`; `# 4` when the designator is unknown (Appendix C2); the
/// designator alone when there is no number (`REAR`).
fn unit_text(t: Option<UnitType>, number: Option<&str>) -> Option<String> {
    match (t, number.map(plain).filter(|n| !n.is_empty())) {
        (Some(t), Some(n)) => Some(format!("{} {n}", t.abbr())),
        (Some(t), None) => Some(t.abbr().to_string()),
        (None, Some(n)) => Some(format!("# {n}")),
        (None, None) => None,
    }
}

/// Drop punctuation Pub 28 does not use, keeping the hyphen and slash
/// (`123-125`, `1/2`); uppercase; collapse whitespace.
fn plain(s: &str) -> String {
    let kept: String = s
        .chars()
        .filter(|c| {
            !matches!(
                c,
                '.' | ',' | '\'' | '\u{2019}' | '"' | ';' | ':' | '(' | ')'
            )
        })
        .flat_map(char::to_uppercase)
        .collect();
    kept.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn join<'a>(parts: impl IntoIterator<Item = &'a Option<String>>) -> String {
    parts
        .into_iter()
        .filter_map(|p| p.as_deref())
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::Parser;

    fn usps(input: &str) -> (String, String, String) {
        let u = Parser::new().parse(input).unwrap().to_usps();
        (
            u.line1.clone().unwrap_or_default(),
            u.line2.clone().unwrap_or_default(),
            u.last_line(),
        )
    }

    #[test]
    fn street_with_unit_on_one_line() {
        let (l1, l2, last) = usps("123 north main street apt 4, miami fl 33101-1234");
        assert_eq!(l1, "123 N MAIN ST APT 4");
        assert_eq!(l2, "");
        assert_eq!(last, "MIAMI FL 33101-1234");
    }

    #[test]
    fn unknown_designator_uses_pound() {
        assert_eq!(usps("4158 Inverrary Dr #309").0, "4158 INVERRARY DR # 309");
        assert_eq!(
            usps("4158 Inverrary Dr Apt #309").0,
            "4158 INVERRARY DR APT 309"
        );
    }

    #[test]
    fn long_unit_moves_to_line2() {
        let (l1, l2, _) = usps(
            "12345 North Abernathy Greenbriar Boulevard Southwest Suite 12044, Miami FL 33101",
        );
        assert_eq!(l1, "12345 N ABERNATHY GREENBRIAR BLVD SW");
        assert_eq!(l2, "STE 12044");
    }

    #[test]
    fn special_forms() {
        assert_eq!(usps("P.O. Box 12, Tampa FL 33601").0, "PO BOX 12");
        assert_eq!(usps("Firm Caller 4410").0, "PO BOX 4410");
        assert_eq!(usps("Star Route 68 Box 23A").0, "HC 68 BOX 23A");
        assert_eq!(usps("RFD 2 Box 152").0, "RR 2 BOX 152");
        assert_eq!(
            usps("General Delivery, Tampa FL 33601").0,
            "GENERAL DELIVERY"
        );
        assert_eq!(
            usps("PSC 1650 Box 10, APO AE 09001"),
            ("PSC 1650 BOX 10".into(), "".into(), "APO AE 09001".into())
        );
        assert_eq!(usps("US 41 SW").0, "US HIGHWAY 41 SW");
        assert_eq!(usps("farm to market 1200").0, "FM 1200");
    }

    #[test]
    fn partial_addresses_leave_parts_out() {
        let (l1, _, last) = usps("123 Main St, Miami FL");
        assert_eq!((l1.as_str(), last.as_str()), ("123 MAIN ST", "MIAMI FL"));
        assert_eq!(
            usps("Tampa FL 33601"),
            ("".into(), "".into(), "TAMPA FL 33601".into())
        );
    }

    #[test]
    fn punctuation_is_dropped() {
        assert_eq!(
            usps("1 O'Brien St., St. Petersburg, FL 33701").0,
            "1 OBRIEN ST"
        );
        assert_eq!(
            usps("1 O'Brien St., St. Petersburg, FL 33701").2,
            "ST PETERSBURG FL 33701"
        );
    }
}
