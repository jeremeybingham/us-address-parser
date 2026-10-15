//! RESO Data Dictionary 2.0 output. Field names, types, maximum lengths and
//! lookups are from the Property resource, group Location / Address; the
//! mapping rules and their sources are in `docs/reso-mapping.md`.
//!
//! This formats a parse; it does not check that the address exists or is
//! deliverable, and it does not check values against a server's lookups.

use super::usps::{join, plain};
use crate::types::{ParsedAddress, UnitType};

/// Address fields of the RESO Data Dictionary 2.0 Property resource.
///
/// A field the parse did not find is `None`. Values are uppercase, in the
/// same forms as [`UspsAddress`](crate::UspsAddress). Field names in
/// [`ResoAddress::fields`] are the Data Dictionary's standard names.
///
/// Not emitted, because nothing is parsed for them: `Country`,
/// `PostalCity`, `StreetSuffixModifier`, `StreetAdditionalInfo`
/// (`docs/reso-mapping.md`, "Fields not emitted").
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResoAddress {
    /// `StreetNumber`: `123`, `123-125`, `123 1/2`, `12A`.
    pub street_number: Option<String>,
    /// `StreetNumberNumeric`: the leading digits of the street number.
    pub street_number_numeric: Option<u64>,
    /// `StreetDirPrefix`: a `StreetDirection` lookup value (`N`, `NE`, ...).
    pub street_dir_prefix: Option<String>,
    /// `StreetName`. Also holds what has no field of its own: a highway name
    /// with its prefix and route, a PO Box, rural route, military box or
    /// `GENERAL DELIVERY` line.
    pub street_name: Option<String>,
    /// `StreetSuffix`: the Pub 28 abbreviation (`ST`). The Data Dictionary
    /// defines no standard values for this field.
    pub street_suffix: Option<String>,
    /// `StreetDirSuffix`: a `StreetDirection` lookup value.
    pub street_dir_suffix: Option<String>,
    /// `UnitNumber`: the designator with the number (`APT 4`, `#55`), as the
    /// Data Dictionary's examples write it; a second unit follows the first.
    pub unit_number: Option<String>,
    /// `City`. For an APO, FPO or DPO address, that post office.
    pub city: Option<String>,
    /// `StateOrProvince`: the two-letter code.
    pub state_or_province: Option<String>,
    /// `PostalCode`: the five-digit ZIP.
    pub postal_code: Option<String>,
    /// `PostalCodePlus4`: the four digits after the hyphen.
    pub postal_code_plus4: Option<String>,
    /// `UnparsedAddress`: `LINE1, CITY, ST ZIP`, with missing parts left out.
    pub unparsed_address: Option<String>,
}

/// Longest `StreetNumberNumeric` the Data Dictionary suggests, in digits.
const STREET_NUMBER_NUMERIC_MAX_DIGITS: usize = 10;

impl ResoAddress {
    /// The string-valued fields that are set, as `(Data Dictionary name,
    /// value)` pairs in Data Dictionary address order. `StreetNumberNumeric`
    /// is included as its decimal text; it is a number in the Data
    /// Dictionary, so convert it back when building a typed payload.
    pub fn fields(&self) -> Vec<(&'static str, String)> {
        let numeric = self.street_number_numeric.map(|n| n.to_string());
        [
            ("StreetNumber", &self.street_number),
            ("StreetNumberNumeric", &numeric),
            ("StreetDirPrefix", &self.street_dir_prefix),
            ("StreetName", &self.street_name),
            ("StreetSuffix", &self.street_suffix),
            ("StreetDirSuffix", &self.street_dir_suffix),
            ("UnitNumber", &self.unit_number),
            ("City", &self.city),
            ("StateOrProvince", &self.state_or_province),
            ("PostalCode", &self.postal_code),
            ("PostalCodePlus4", &self.postal_code_plus4),
            ("UnparsedAddress", &self.unparsed_address),
        ]
        .into_iter()
        .filter_map(|(k, v)| v.as_ref().map(|v| (k, v.clone())))
        .collect()
    }

    /// Names of the fields whose value is longer than the Data Dictionary's
    /// suggested maximum length. Values are never cut: a caller that must
    /// fit a column decides what to do with these.
    pub fn length_overruns(&self) -> Vec<&'static str> {
        self.fields()
            .into_iter()
            .filter(|(k, v)| max_length(k).is_some_and(|m| v.chars().count() > m))
            .map(|(k, _)| k)
            .collect()
    }
}

/// Suggested maximum lengths, Data Dictionary 2.0 Property fields.
fn max_length(field: &str) -> Option<usize> {
    Some(match field {
        "StreetNumber" | "StreetSuffix" | "UnitNumber" => 25,
        "StreetNumberNumeric" => STREET_NUMBER_NUMERIC_MAX_DIGITS,
        "StreetDirPrefix" | "StreetDirSuffix" => 15,
        "StreetName" | "City" => 50,
        "StateOrProvince" => 2,
        "PostalCode" => 10,
        "PostalCodePlus4" => 4,
        "UnparsedAddress" => 255,
        _ => return None,
    })
}

impl ParsedAddress {
    /// The address as RESO Data Dictionary 2.0 Property address fields.
    /// Components the parse did not find are `None`;
    /// [`Diagnostics`](crate::Diagnostics) says what was missing or unparsed.
    pub fn to_reso(&self) -> ResoAddress {
        let usps = self.to_usps();
        let street_number = self.street_number.as_deref().map(plain);

        // A dual address (Pub 28 section 215) gives the street: RESO fields
        // describe the property. The USPS output writes the PO Box.
        let dual = self.is_dual();
        let street_name = if dual {
            self.street_name_text()
        } else {
            self.special_delivery_line()
                .or_else(|| self.street_name_text())
        };

        let unit_number = join([
            &unit_text(self.unit_type, self.unit_number.as_deref()),
            &unit_text(self.second_unit_type, self.second_unit_number.as_deref()),
        ]);

        let last = join([&usps.state, &usps.zip]);
        let line1 = if dual {
            self.street_line()
                .map(|s| join([&Some(s), &Some(self.units_text())]))
        } else {
            usps.line1.clone()
        };
        let unparsed_address = [line1, usps.city.clone(), Some(last)]
            .into_iter()
            .flatten()
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>()
            .join(", ");

        ResoAddress {
            street_number_numeric: street_number.as_deref().and_then(leading_number),
            street_number,
            street_dir_prefix: self.street_dir_prefix.map(|d| d.abbr().to_string()),
            street_name: street_name.filter(|n| !n.is_empty()),
            street_suffix: self.street_suffix.map(|s| s.abbr().to_string()),
            street_dir_suffix: self.street_dir_suffix.map(|d| d.abbr().to_string()),
            unit_number: (!unit_number.is_empty()).then_some(unit_number),
            city: usps.city,
            state_or_province: usps.state,
            postal_code: self.zip5.clone(),
            postal_code_plus4: self.zip4.clone(),
            unparsed_address: (!unparsed_address.is_empty()).then_some(unparsed_address),
        }
    }
}

/// `APT 4`, `#55` (the Data Dictionary's `UnitNumber` examples write the
/// pound sign without a space), or the designator alone (`REAR`).
fn unit_text(t: Option<UnitType>, number: Option<&str>) -> Option<String> {
    match (t, number.map(plain).filter(|n| !n.is_empty())) {
        (Some(t), Some(n)) => Some(format!("{} {n}", t.abbr())),
        (Some(t), None) => Some(t.abbr().to_string()),
        (None, Some(n)) => Some(format!("#{n}")),
        (None, None) => None,
    }
}

/// The integer a street number starts with: `123` for `123-125` and `12A`.
/// `None` when it starts with no digit or has more digits than
/// `StreetNumberNumeric` suggests.
fn leading_number(s: &str) -> Option<u64> {
    let digits: String = s.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() || digits.len() > STREET_NUMBER_NUMERIC_MAX_DIGITS {
        return None;
    }
    digits.parse().ok()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::{Parser, ResoAddress};

    fn reso(input: &str) -> ResoAddress {
        Parser::new().parse(input).unwrap().to_reso()
    }

    fn s(v: &str) -> Option<String> {
        Some(v.to_string())
    }

    #[test]
    fn full_street_address() {
        let r = reso("123 north main street apt 4, miami fl 33101-1234");
        assert_eq!(
            r,
            ResoAddress {
                street_number: s("123"),
                street_number_numeric: Some(123),
                street_dir_prefix: s("N"),
                street_name: s("MAIN"),
                street_suffix: s("ST"),
                street_dir_suffix: None,
                unit_number: s("APT 4"),
                city: s("MIAMI"),
                state_or_province: s("FL"),
                postal_code: s("33101"),
                postal_code_plus4: s("1234"),
                unparsed_address: s("123 N MAIN ST APT 4, MIAMI, FL 33101-1234"),
            }
        );
        assert!(r.length_overruns().is_empty());
    }

    #[test]
    fn fields_use_data_dictionary_names_in_order() {
        let f = reso("123 N Main St SW Apt 4, Miami FL 33101").fields();
        let names: Vec<_> = f.iter().map(|(k, _)| *k).collect();
        assert_eq!(
            names,
            [
                "StreetNumber",
                "StreetNumberNumeric",
                "StreetDirPrefix",
                "StreetName",
                "StreetSuffix",
                "StreetDirSuffix",
                "UnitNumber",
                "City",
                "StateOrProvince",
                "PostalCode",
                "UnparsedAddress"
            ]
        );
        assert_eq!(f[5].1, "SW");
    }

    #[test]
    fn unit_forms() {
        assert_eq!(reso("4158 Inverrary Dr #309").unit_number, s("#309"));
        assert_eq!(reso("4158 Inverrary Dr Apt #309").unit_number, s("APT 309"));
        assert_eq!(reso("100 Main St Ste 5").unit_number, s("STE 5"));
    }

    #[test]
    fn street_number_numeric_is_the_leading_integer() {
        let r = reso("123-125 Main St");
        assert_eq!(r.street_number, s("123-125"));
        assert_eq!(r.street_number_numeric, Some(123));
        assert_eq!(reso("12A Main St").street_number_numeric, Some(12));
        assert_eq!(reso("123 1/2 Main St").street_number_numeric, Some(123));
        let none = reso("Main St");
        assert_eq!(none.street_number, None);
        assert_eq!(none.street_number_numeric, None);
    }

    #[test]
    fn highway_name_is_folded_into_street_name() {
        let r = reso("100 Kentucky State Highway 625");
        assert_eq!(r.street_number, s("100"));
        assert_eq!(r.street_name, s("KY STATE HIGHWAY 625"));
        let r = reso("US 41 SW");
        assert_eq!(r.street_name, s("US HIGHWAY 41"));
        assert_eq!(r.street_dir_suffix, s("SW"));
    }

    #[test]
    fn forms_with_no_street_are_folded_into_street_name() {
        let r = reso("P.O. Box 12, Tampa FL 33601");
        assert_eq!(r.street_number, None);
        assert_eq!(r.street_name, s("PO BOX 12"));
        assert_eq!(r.unparsed_address, s("PO BOX 12, TAMPA, FL 33601"));
        assert_eq!(reso("RFD 2 Box 152").street_name, s("RR 2 BOX 152"));
        assert_eq!(
            reso("General Delivery, Tampa FL 33601").street_name,
            s("GENERAL DELIVERY")
        );
        let m = reso("PSC 1650 Box 10, APO AE 09001");
        assert_eq!(m.street_name, s("PSC 1650 BOX 10"));
        assert_eq!(m.city, s("APO"));
        assert_eq!(m.state_or_province, s("AE"));
        assert_eq!(m.postal_code, s("09001"));
    }

    #[test]
    fn partial_addresses_leave_fields_out() {
        let r = reso("Tampa FL 33601");
        assert_eq!(r.street_name, None);
        assert_eq!(r.unparsed_address, s("TAMPA, FL 33601"));
        assert_eq!(reso("123 Main St").unparsed_address, s("123 MAIN ST"));
    }

    #[test]
    fn an_address_with_nothing_found_gives_empty_fields() {
        let empty = crate::ParsedAddress::empty(String::new(), crate::InputKind::SingleLine, None);
        assert_eq!(empty.to_reso(), ResoAddress::default());
        assert!(empty.to_reso().fields().is_empty());
    }

    #[test]
    fn overlong_values_are_reported_not_cut() {
        let long = "x".repeat(60);
        let r = reso(&format!("1 {long} St, Miami FL 33101"));
        assert_eq!(r.street_name.as_deref().map(str::len), Some(60));
        assert_eq!(r.length_overruns(), ["StreetName"]);
    }
}
