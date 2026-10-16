//! JSON and flat-column views of a [`ParsedAddress`]. Read-only: the CLI
//! formats what the parser produced and never re-parses it.

use addr_core::{Highway, KeyOptions, ParsedAddress, PlaceKind, UsaddressTag};
use serde_json::{json, Map, Value};

/// Parsed components, by `ParsedAddress` field name. Components the parse
/// did not find are left out.
fn components(a: &ParsedAddress) -> Map<String, Value> {
    let mut m = Map::new();
    let mut put = |k: &str, v: Option<String>| {
        if let Some(v) = v {
            m.insert(k.to_string(), Value::String(v));
        }
    };
    put("street_number", a.street_number.clone());
    put(
        "street_dir_prefix",
        a.street_dir_prefix.map(|d| d.abbr().into()),
    );
    put("street_name", a.street_name.clone());
    put("street_suffix", a.street_suffix.map(|s| s.abbr().into()));
    put(
        "street_dir_suffix",
        a.street_dir_suffix.map(|d| d.abbr().into()),
    );
    put("highway", a.highway.as_ref().map(Highway::name));
    put("unit_type", a.unit_type.map(|u| u.abbr().into()));
    put("unit_number", a.unit_number.clone());
    put(
        "second_unit_type",
        a.second_unit_type.map(|u| u.abbr().into()),
    );
    put("second_unit_number", a.second_unit_number.clone());
    put("po_box", a.po_box.clone());
    put(
        "rural_route",
        a.rural_route.as_ref().map(|r| {
            let b = r
                .box_number
                .as_deref()
                .map_or(String::new(), |b| format!(" BOX {b}"));
            format!("{} {}{b}", r.kind.abbr(), r.route)
        }),
    );
    put(
        "general_delivery",
        a.general_delivery.then(|| "GENERAL DELIVERY".into()),
    );
    put(
        "military",
        a.military.as_ref().map(|m| {
            let b = m
                .box_number
                .as_deref()
                .map_or(String::new(), |b| format!(" BOX {b}"));
            format!("{} {}{b}", m.kind.abbr(), m.number)
        }),
    );
    put(
        "military_post_office",
        a.military_post_office.map(|m| m.abbr().into()),
    );
    put("attention", a.attention.clone());
    put("building", a.building.clone());
    put("city", a.city.clone());
    put("state", a.state.map(|s| s.to_string()));
    put("zip5", a.zip5.clone());
    put("zip4", a.zip4.clone());
    m
}

fn warning_codes(a: &ParsedAddress) -> Vec<&'static str> {
    a.diagnostics.warnings.iter().map(|w| w.code()).collect()
}

/// The full record printed by `addr parse`.
pub fn record(a: &ParsedAddress) -> Value {
    let usps = a.to_usps();
    let reso: Map<String, Value> = a
        .to_reso()
        .fields()
        .into_iter()
        .map(|(k, v)| (k.to_string(), Value::String(v)))
        .collect();
    json!({
        "input": a.source.original,
        "components": components(a),
        "warnings": warning_codes(a),
        "unparsed": a.diagnostics.unparsed,
        "key": a.key(KeyOptions::property()),
        "key_unit": a.key(KeyOptions::unit()),
        "slug": a.slug(),
        "usps": {
            "line1": usps.line1,
            "line2": usps.line2,
            "last_line": usps.last_line(),
        },
        "reso": reso,
        "place": a.place.map(|m| json!({
            "name": m.place.name(),
            "state": m.place.state(),
            "county": m.place.county(),
            "kind": match m.place.kind() {
                PlaceKind::Municipality => "municipality",
                PlaceKind::Place => "place",
            },
            "source": m.source.code(),
        })),
    })
}

/// The record printed by `addr parse --usaddress`: each token as
/// `[text, label or null, start, end]`, with byte offsets into its field (or
/// the input, for one line).
pub fn usaddress_record(input: &str, tags: &[UsaddressTag]) -> Value {
    let tags: Vec<Value> = tags
        .iter()
        .map(|t| json!([t.text, t.label, t.span.start, t.span.end]))
        .collect();
    json!({ "input": input, "tags": tags })
}

/// Columns `addr parse-file` appends, in order.
pub const COLUMNS: [&str; 30] = [
    "street_number",
    "street_dir_prefix",
    "street_name",
    "street_suffix",
    "street_dir_suffix",
    "unit_type",
    "unit_number",
    "second_unit_type",
    "second_unit_number",
    "city",
    "state",
    "zip5",
    "zip4",
    "place",
    "place_source",
    "highway",
    "po_box",
    "rural_route",
    "general_delivery",
    "military",
    "military_post_office",
    "attention",
    "building",
    "usps_line1",
    "usps_last_line",
    "key",
    "key_unit",
    "slug",
    "warnings",
    "unparsed",
];

/// One value per [`COLUMNS`] entry; empty when absent.
pub fn row(a: &ParsedAddress) -> Vec<String> {
    let c = components(a);
    let usps = a.to_usps();
    let get = |k: &str| c.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    COLUMNS
        .iter()
        .map(|&col| match col {
            "usps_line1" => usps.line1.clone().unwrap_or_default(),
            "usps_last_line" => usps.last_line(),
            "key" => a.key(KeyOptions::property()).unwrap_or_default(),
            "key_unit" => a.key(KeyOptions::unit()).unwrap_or_default(),
            "slug" => a.slug().unwrap_or_default(),
            "place" => a
                .place
                .map(|m| m.place.name().to_string())
                .unwrap_or_default(),
            "place_source" => a
                .place
                .map(|m| m.source.code().to_string())
                .unwrap_or_default(),
            "warnings" => warning_codes(a).join(";"),
            "unparsed" => a.diagnostics.unparsed.clone().unwrap_or_default(),
            other => get(other),
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use addr_core::{Fields, Parser};

    /// Every component `components()` can print has a `parse-file` column.
    /// The inputs between them fill every component; extend them when a
    /// component is added.
    #[test]
    fn every_component_has_a_column() {
        let p = Parser::new();
        let mut parsed: Vec<ParsedAddress> = [
            "123 N MAIN ST E APT 4 PMB 9, MIAMI, FL 33101-1234",
            "100 HWY 66 FRONTAGE RD, TULSA OK 74101",
            "RR 2 BOX 152, AXTELL NE 68924",
            "GENERAL DELIVERY, JUNEAU AK 99801",
            "PSC 1650 BOX 10, APO AE 09001",
            "1679 E ORANGETHORPE AVE PO BOX 365, ATWOOD, CA 92811",
        ]
        .iter()
        .map(|s| p.parse(s).unwrap())
        .collect();
        for line2 in ["ANNEX BUILDING", "ATTN LAW DEPT"] {
            let f = Fields {
                line1: Some("323 NORRISTOWN ROAD".into()),
                line2: Some(line2.into()),
                city: Some("OCALA".into()),
                state: Some("FL".into()),
                zip: Some("34470".into()),
            };
            parsed.push(p.parse_fields(&f).unwrap());
        }
        let mut seen = std::collections::BTreeSet::new();
        for a in &parsed {
            for k in components(a).keys() {
                assert!(COLUMNS.contains(&k.as_str()), "no column for `{k}`");
                seen.insert(k.clone());
            }
            assert_eq!(row(a).len(), COLUMNS.len());
        }
        assert_eq!(
            seen.len(),
            21,
            "inputs no longer cover every component: {seen:?}"
        );
    }

    #[test]
    fn second_unit_is_in_the_row() {
        let a = Parser::new().parse("123 MAIN ST STE 201 PMB 308").unwrap();
        let r = row(&a);
        let col = |n: &str| r[COLUMNS.iter().position(|c| *c == n).unwrap()].as_str();
        assert_eq!((col("unit_type"), col("unit_number")), ("STE", "201"));
        assert_eq!(
            (col("second_unit_type"), col("second_unit_number")),
            ("PMB", "308")
        );
    }
}
