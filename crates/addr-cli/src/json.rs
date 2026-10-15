//! JSON and flat-column views of a [`ParsedAddress`]. Read-only: the CLI
//! formats what the parser produced and never re-parses it.

use addr_core::{Highway, KeyOptions, ParsedAddress};
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
    })
}

/// Columns `addr parse-file` appends, in order.
pub const COLUMNS: [&str; 23] = [
    "street_number",
    "street_dir_prefix",
    "street_name",
    "street_suffix",
    "street_dir_suffix",
    "unit_type",
    "unit_number",
    "city",
    "state",
    "zip5",
    "zip4",
    "highway",
    "po_box",
    "rural_route",
    "military",
    "attention",
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
            "warnings" => warning_codes(a).join(";"),
            "unparsed" => a.diagnostics.unparsed.clone().unwrap_or_default(),
            other => get(other),
        })
        .collect()
}
