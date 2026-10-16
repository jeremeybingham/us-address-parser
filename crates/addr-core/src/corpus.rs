//! Golden-corpus loader and runner, behind the `corpus` feature. The format
//! and labeling conventions are in `tests/corpus/README.md`.
//!
//! Every case is parsed and compared field by field. The corpus may be
//! ahead of the parser, so a failing case alone is not an error: callers
//! compare the passing set with the recorded baseline
//! (`tests/corpus/passing.txt`) and treat a case that used to pass and now
//! fails as a regression.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::{
    Directional, Fields, Highway, KeyOptions, Lexicons, MatchKind, ParseOptions, ParsedAddress,
    Parser, Warning,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub id: String,
    input: Option<String>,
    lines: Option<Vec<String>>,
    fields: Option<CaseFields>,
    expect: Expect,
    /// `ParseOptions::default_state` for this case; absent means none.
    default_state: Option<String>,
    #[serde(default)]
    warnings: Vec<String>,
    /// Property-level key (`KeyOptions::property()`), or `"none"` when the
    /// address has no key. Unlike `expect`, absent means not checked.
    key: Option<String>,
    /// Unit-level key (`KeyOptions::unit()`); same conventions as `key`.
    key_unit: Option<String>,
    /// Expected `slug()`, or `"none"`; absent means not checked.
    slug: Option<String>,
    /// Expected `to_usps()` output; absent means not checked.
    usps: Option<ExpectUsps>,
    /// Expected `to_reso()` fields by Data Dictionary name; a field not
    /// listed must be empty. Absent means not checked.
    reso: Option<BTreeMap<String, String>>,
    /// Expected `ParsedAddress::place` as `"NAME (source)"`, such as
    /// `"WEST PALM BEACH (name)"`, or `"none"`. Absent means not checked.
    place: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    note: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectUsps {
    line1: Option<String>,
    line2: Option<String>,
    /// `CITY ST ZIP`, as `UspsAddress::last_line`.
    last_line: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct CaseFields {
    line1: Option<String>,
    line2: Option<String>,
    city: Option<String>,
    state: Option<String>,
    zip: Option<String>,
}

/// Expected components. An absent key means the component must be empty.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Expect {
    street_number: Option<String>,
    street_dir_prefix: Option<String>,
    street_name: Option<String>,
    street_suffix: Option<String>,
    street_dir_suffix: Option<String>,
    highway: Option<ExpectHighway>,
    unit_type: Option<String>,
    unit_number: Option<String>,
    second_unit_type: Option<String>,
    second_unit_number: Option<String>,
    po_box: Option<String>,
    rural_route: Option<ExpectRoute>,
    #[serde(default)]
    general_delivery: bool,
    military: Option<ExpectMilitary>,
    military_post_office: Option<String>,
    attention: Option<String>,
    building: Option<String>,
    city: Option<String>,
    state: Option<String>,
    zip5: Option<String>,
    zip4: Option<String>,
    unparsed: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectRoute {
    /// "RR" or "HC".
    kind: String,
    route: String,
    #[serde(rename = "box")]
    box_number: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectHighway {
    /// State code named in the highway name (`KY STATE HIGHWAY 1`).
    state: Option<String>,
    /// Standard prefix: "COUNTY ROAD", "HIGHWAY", "FM".
    prefix: String,
    route: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectMilitary {
    /// "CPR", "OPC", "PSC", "UPR" or "UNIT".
    kind: String,
    number: String,
    #[serde(rename = "box")]
    box_number: Option<String>,
}

/// Compared fields, in report order. `warnings` compares as a set.
const FIELDS: [&str; 29] = [
    "street_number",
    "street_dir_prefix",
    "street_name",
    "street_suffix",
    "street_dir_suffix",
    "highway",
    "unit_type",
    "unit_number",
    "second_unit_type",
    "second_unit_number",
    "po_box",
    "rural_route",
    "general_delivery",
    "military",
    "military_post_office",
    "attention",
    "building",
    "city",
    "state",
    "zip5",
    "zip4",
    "unparsed",
    "warnings",
    "key",
    "key_unit",
    "slug",
    "usps",
    "reso",
    "place",
];

/// Fields checked only for the cases that give them.
const OPTIONAL_FIELDS: [&str; 6] = ["key", "key_unit", "slug", "usps", "reso", "place"];

/// Data Dictionary 2.0 names a case's `reso` object may use.
const RESO_FIELDS: [&str; 12] = [
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
    "PostalCodePlus4",
    "UnparsedAddress",
];

type Values = BTreeMap<&'static str, Option<String>>;

fn flag(b: bool) -> Option<String> {
    b.then(|| "true".to_string())
}

fn route_str(kind: &str, route: &str, box_number: Option<&str>) -> String {
    match box_number {
        Some(b) => format!("{kind} {route} BOX {b}"),
        None => format!("{kind} {route}"),
    }
}

fn warnings_str<'a>(codes: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let set: BTreeSet<&str> = codes.into_iter().collect();
    if set.is_empty() {
        None
    } else {
        Some(set.into_iter().collect::<Vec<_>>().join(","))
    }
}

fn usps_str(l1: &Option<String>, l2: &Option<String>, last: &Option<String>) -> String {
    let f = |o: &Option<String>| o.clone().unwrap_or_default();
    format!("[{}] [{}] [{}]", f(l1), f(l2), f(last))
}

/// `Name=value` pairs sorted by name, so the order a case lists them in
/// does not matter.
fn reso_str<'a>(pairs: impl IntoIterator<Item = (&'a str, &'a str)>) -> String {
    let sorted: BTreeMap<&str, &str> = pairs.into_iter().collect();
    sorted
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("; ")
}

fn expected_values(c: &Case) -> Values {
    let e = &c.expect;
    let mut v = Values::new();
    v.insert("street_number", e.street_number.clone());
    v.insert("street_dir_prefix", e.street_dir_prefix.clone());
    v.insert("street_name", e.street_name.clone());
    v.insert("street_suffix", e.street_suffix.clone());
    v.insert("street_dir_suffix", e.street_dir_suffix.clone());
    v.insert(
        "highway",
        e.highway.as_ref().map(|h| match &h.state {
            Some(s) => format!("{s} {} {}", h.prefix, h.route),
            None => format!("{} {}", h.prefix, h.route),
        }),
    );
    v.insert("unit_type", e.unit_type.clone());
    v.insert("unit_number", e.unit_number.clone());
    v.insert("second_unit_type", e.second_unit_type.clone());
    v.insert("second_unit_number", e.second_unit_number.clone());
    v.insert("po_box", e.po_box.clone());
    v.insert(
        "rural_route",
        e.rural_route
            .as_ref()
            .map(|r| route_str(&r.kind, &r.route, r.box_number.as_deref())),
    );
    v.insert("general_delivery", flag(e.general_delivery));
    v.insert(
        "military",
        e.military
            .as_ref()
            .map(|m| route_str(&m.kind, &m.number, m.box_number.as_deref())),
    );
    v.insert("military_post_office", e.military_post_office.clone());
    v.insert("attention", e.attention.clone());
    v.insert("building", e.building.clone());
    v.insert("city", e.city.clone());
    v.insert("state", e.state.clone());
    v.insert("zip5", e.zip5.clone());
    v.insert("zip4", e.zip4.clone());
    v.insert("unparsed", e.unparsed.clone());
    v.insert(
        "warnings",
        warnings_str(c.warnings.iter().map(String::as_str)),
    );
    if let Some(k) = &c.key {
        v.insert("key", Some(k.clone()));
    }
    if let Some(k) = &c.key_unit {
        v.insert("key_unit", Some(k.clone()));
    }
    if let Some(k) = &c.slug {
        v.insert("slug", Some(k.clone()));
    }
    if let Some(u) = &c.usps {
        v.insert("usps", Some(usps_str(&u.line1, &u.line2, &u.last_line)));
    }
    if let Some(r) = &c.reso {
        v.insert(
            "reso",
            Some(reso_str(r.iter().map(|(k, v)| (k.as_str(), v.as_str())))),
        );
    }
    if let Some(p) = &c.place {
        v.insert("place", Some(p.clone()));
    }
    v
}

fn actual_values(a: &ParsedAddress) -> Values {
    let mut v = Values::new();
    v.insert("street_number", a.street_number.clone());
    v.insert(
        "street_dir_prefix",
        a.street_dir_prefix.map(|d| d.abbr().to_string()),
    );
    v.insert("street_name", a.street_name.clone());
    v.insert(
        "street_suffix",
        a.street_suffix.map(|s| s.abbr().to_string()),
    );
    v.insert(
        "street_dir_suffix",
        a.street_dir_suffix.map(|d| d.abbr().to_string()),
    );
    v.insert("highway", a.highway.as_ref().map(Highway::name));
    v.insert("unit_type", a.unit_type.map(|u| u.abbr().to_string()));
    v.insert("unit_number", a.unit_number.clone());
    v.insert(
        "second_unit_type",
        a.second_unit_type.map(|u| u.abbr().to_string()),
    );
    v.insert("second_unit_number", a.second_unit_number.clone());
    v.insert("po_box", a.po_box.clone());
    v.insert(
        "rural_route",
        a.rural_route
            .as_ref()
            .map(|r| route_str(r.kind.abbr(), &r.route, r.box_number.as_deref())),
    );
    v.insert("general_delivery", flag(a.general_delivery));
    v.insert(
        "military",
        a.military
            .as_ref()
            .map(|m| route_str(m.kind.abbr(), &m.number, m.box_number.as_deref())),
    );
    v.insert(
        "military_post_office",
        a.military_post_office.map(|m| m.abbr().to_string()),
    );
    v.insert("attention", a.attention.clone());
    v.insert("building", a.building.clone());
    v.insert("city", a.city.clone());
    v.insert("state", a.state.map(|s| s.code().to_string()));
    v.insert("zip5", a.zip5.clone());
    v.insert("zip4", a.zip4.clone());
    v.insert("unparsed", a.diagnostics.unparsed.clone());
    v.insert(
        "warnings",
        warnings_str(a.diagnostics.warnings.iter().map(Warning::code)),
    );
    let key = |o| Some(a.key(o).unwrap_or_else(|| "none".to_string()));
    v.insert("key", key(KeyOptions::property()));
    v.insert("key_unit", key(KeyOptions::unit()));
    v.insert("slug", Some(a.slug().unwrap_or_else(|| "none".to_string())));
    let u = a.to_usps();
    let last = u.last_line();
    v.insert(
        "usps",
        Some(usps_str(
            &u.line1,
            &u.line2,
            &(!last.is_empty()).then_some(last),
        )),
    );
    let reso = a.to_reso().fields();
    v.insert(
        "reso",
        Some(reso_str(reso.iter().map(|(k, v)| (*k, v.as_str())))),
    );
    v.insert(
        "place",
        Some(a.place.map_or("none".to_string(), |m| {
            format!("{} ({})", m.place.name(), m.source.code())
        })),
    );
    v
}

/// A loaded and validated corpus: `(file name, case)` pairs in file and line
/// order.
pub struct Corpus {
    cases: Vec<(String, Case)>,
}

/// Load every `*.jsonl` file in `dir` and check the format. Every problem
/// found is returned, not just the first.
pub fn load(dir: &Path) -> Result<Corpus, Vec<String>> {
    let mut errors = Vec::new();
    let mut files: Vec<PathBuf> = match fs::read_dir(dir) {
        Ok(rd) => rd
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
            .collect(),
        Err(e) => return Err(vec![format!("{}: {e}", dir.display())]),
    };
    files.sort();
    let mut cases = Vec::new();
    for path in files {
        let name = path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let text = match fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                errors.push(format!("{name}: {e}"));
                continue;
            }
        };
        for (i, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<Case>(line) {
                Ok(c) => cases.push((name.clone(), c)),
                Err(e) => errors.push(format!("{name}:{}: {e}", i + 1)),
            }
        }
    }
    validate(&cases, &mut errors);
    if errors.is_empty() {
        Ok(Corpus { cases })
    } else {
        Err(errors)
    }
}

/// Check what serde cannot: one input form per case, unique ids, and
/// expected values that are real lexicon entries and warning codes.
fn validate(cases: &[(String, Case)], errors: &mut Vec<String>) {
    let lex = Lexicons::builtin();
    let suffixes: BTreeSet<_> = lex.suffixes().map(|s| s.abbr()).collect();
    let units: BTreeSet<_> = lex.units().map(|u| u.abbr()).collect();
    let states: BTreeSet<_> = lex.states().map(|s| s.code()).collect();
    let dirs: BTreeSet<_> = Directional::ALL.iter().map(|d| d.abbr()).collect();
    let codes: BTreeSet<_> = Warning::ALL.iter().map(Warning::code).collect();

    let mut ids = BTreeSet::new();
    for (file, c) in cases {
        let at = format!("{file}: {}", c.id);
        if !ids.insert(c.id.as_str()) {
            errors.push(format!("{at}: duplicate id"));
        }
        let forms = [c.input.is_some(), c.lines.is_some(), c.fields.is_some()];
        if forms.iter().filter(|&&b| b).count() != 1 {
            errors.push(format!("{at}: needs exactly one of input, lines, fields"));
        }
        let e = &c.expect;
        let checks = [
            ("street_suffix", &e.street_suffix, &suffixes),
            ("unit_type", &e.unit_type, &units),
            ("second_unit_type", &e.second_unit_type, &units),
            ("state", &e.state, &states),
            ("street_dir_prefix", &e.street_dir_prefix, &dirs),
            ("street_dir_suffix", &e.street_dir_suffix, &dirs),
        ];
        for (field, value, allowed) in checks {
            if let Some(v) = value {
                if !allowed.contains(v.as_str()) {
                    errors.push(format!("{at}: {field} {v:?} is not a standard value"));
                }
            }
        }
        if let Some(r) = &e.rural_route {
            if r.kind != "RR" && r.kind != "HC" {
                errors.push(format!("{at}: rural_route kind must be RR or HC"));
            }
        }
        if let Some(h) = &e.highway {
            let standard = lex
                .highway(&h.prefix)
                .filter(|l| l.kind == MatchKind::Standard);
            if standard.is_none() {
                errors.push(format!(
                    "{at}: highway prefix {:?} is not a standard prefix",
                    h.prefix
                ));
            }
            if let Some(s) = &h.state {
                if !states.contains(s.as_str()) {
                    errors.push(format!("{at}: highway state {s:?} is not a state code"));
                }
            }
        }
        if let Some(m) = &e.military {
            if lex.military(&m.kind).map(|l| l.value.abbr()) != Some(m.kind.as_str()) {
                errors.push(format!("{at}: military kind {:?} is not standard", m.kind));
            }
        }
        if let Some(p) = &e.military_post_office {
            if lex.military_post_office(p).map(|l| l.value.abbr()) != Some(p.as_str()) {
                errors.push(format!("{at}: military_post_office {p:?} is not standard"));
            }
        }
        for w in &c.warnings {
            if !codes.contains(w.as_str()) {
                errors.push(format!("{at}: unknown warning code {w:?}"));
            }
        }
        for k in c.reso.iter().flat_map(BTreeMap::keys) {
            if !RESO_FIELDS.contains(&k.as_str()) {
                errors.push(format!("{at}: unknown RESO field {k:?}"));
            }
        }
    }
}

pub(crate) fn parse_case(parser: &Parser, c: &Case) -> Option<ParsedAddress> {
    // A case with its own `default_state` is parsed with it.
    let own;
    let parser = match &c.default_state {
        Some(s) => {
            own = Parser::with_options(ParseOptions {
                default_state: Some(s.clone()),
            })
            .ok()?;
            &own
        }
        None => parser,
    };
    let result = if let Some(s) = &c.input {
        parser.parse(s)
    } else if let Some(lines) = &c.lines {
        // Two lines are the address lines of `Fields`; parse_fields reads
        // them like one line when city, state and ZIP are empty. Longer
        // records are joined the way a single-line caller would write them.
        match lines.as_slice() {
            [l1] => parser.parse(l1),
            [l1, l2] => parser.parse_fields(&Fields {
                line1: Some(l1.clone()),
                line2: Some(l2.clone()),
                ..Fields::default()
            }),
            _ => parser.parse(&lines.join(", ")),
        }
    } else if let Some(f) = &c.fields {
        parser.parse_fields(&Fields {
            line1: f.line1.clone(),
            line2: f.line2.clone(),
            city: f.city.clone(),
            state: f.state.clone(),
            zip: f.zip.clone(),
        })
    } else {
        return None;
    };
    result.ok()
}

fn run_case(parser: &Parser, c: &Case) -> Values {
    match parse_case(parser, c) {
        Some(a) => actual_values(&a),
        None => {
            let mut v: Values = FIELDS.iter().map(|&f| (f, None)).collect();
            v.insert("key", Some("none".to_string()));
            v.insert("key_unit", Some("none".to_string()));
            v.insert("slug", Some("none".to_string()));
            v.insert("usps", Some(usps_str(&None, &None, &None)));
            v.insert("reso", Some(String::new()));
            v.insert("place", Some("none".to_string()));
            v.insert("unparsed", Some("<error>".to_string()));
            v
        }
    }
}

fn show(v: &Option<String>) -> String {
    match v {
        Some(s) => format!("{s:?}"),
        None => "-".to_string(),
    }
}

/// Results of running a corpus.
pub struct Report {
    total: usize,
    passing: BTreeSet<String>,
    ids: BTreeSet<String>,
    /// Per case: the file, id and field differences.
    failures: BTreeMap<String, String>,
    field_ok: BTreeMap<&'static str, usize>,
    field_n: BTreeMap<&'static str, usize>,
}

/// A [`Report`] compared with the recorded baseline.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Comparison {
    /// In the baseline but no longer in the corpus.
    pub stale: Vec<String>,
    /// In the baseline, in the corpus, and now failing.
    pub regressed: Vec<String>,
    /// Passing now, not yet in the baseline.
    pub newly_passing: Vec<String>,
}

impl Comparison {
    /// Whether the baseline is violated (stale or regressed ids).
    pub fn is_failure(&self) -> bool {
        !self.stale.is_empty() || !self.regressed.is_empty()
    }
}

impl Corpus {
    pub fn len(&self) -> usize {
        self.cases.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cases.is_empty()
    }

    /// Each case as the single string a one-line caller would pass: the
    /// `input`, the `lines` joined with `", "`, or the non-empty `fields`
    /// joined the same way. For benchmarks and bulk checks.
    pub fn inputs(&self) -> Vec<String> {
        self.cases
            .iter()
            .map(|(_, c)| {
                if let Some(s) = &c.input {
                    s.clone()
                } else if let Some(l) = &c.lines {
                    l.join(", ")
                } else if let Some(f) = &c.fields {
                    [&f.line1, &f.line2, &f.city, &f.state, &f.zip]
                        .into_iter()
                        .flatten()
                        .filter(|s| !s.is_empty())
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                } else {
                    String::new()
                }
            })
            .filter(|s| !s.trim().is_empty())
            .collect()
    }

    /// Parse every case and compare it with its expected values.
    pub fn run(&self, parser: &Parser) -> Report {
        let mut field_ok: BTreeMap<&'static str, usize> = FIELDS.iter().map(|&f| (f, 0)).collect();
        let mut field_n = field_ok.clone();
        let mut passing = BTreeSet::new();
        let mut failures = BTreeMap::new();
        for (file, c) in &self.cases {
            let want = expected_values(c);
            let got = run_case(parser, c);
            let mut diffs = Vec::new();
            for f in FIELDS {
                if OPTIONAL_FIELDS.contains(&f) && !want.contains_key(f) {
                    continue;
                }
                *field_n.entry(f).or_insert(0) += 1;
                let (w, g) = (
                    want.get(f).cloned().flatten(),
                    got.get(f).cloned().flatten(),
                );
                if w == g {
                    *field_ok.entry(f).or_insert(0) += 1;
                } else {
                    diffs.push(format!("    {f}: want {} got {}", show(&w), show(&g)));
                }
            }
            if diffs.is_empty() {
                passing.insert(c.id.clone());
            } else {
                failures.insert(
                    c.id.clone(),
                    format!("  {file}: {}\n{}", c.id, diffs.join("\n")),
                );
            }
        }
        Report {
            total: self.cases.len(),
            passing,
            ids: self.cases.iter().map(|(_, c)| c.id.clone()).collect(),
            failures,
            field_ok,
            field_n,
        }
    }

    /// Parsing the USPS output of a parsed address must give the same keys.
    /// Returns one message per mismatch.
    pub fn usps_round_trip_mismatches(&self, parser: &Parser) -> Vec<String> {
        let mut bad = Vec::new();
        for (_, c) in &self.cases {
            let Some(a) = parse_case(parser, c) else {
                continue;
            };
            // A dual address's USPS output is its PO Box alone (Pub 28
            // section 215), while its key is the street's.
            if a.diagnostics.has(&Warning::DualAddress) {
                continue;
            }
            let lines = a.to_usps().lines();
            if lines.is_empty() {
                continue;
            }
            let Ok(b) = parser.parse(&lines.join(", ")) else {
                bad.push(format!("{}: reparse failed", c.id));
                continue;
            };
            for o in [KeyOptions::property(), KeyOptions::unit()] {
                if a.key(o) != b.key(o) {
                    bad.push(format!(
                        "{}: {:?} -> {:?}: {:?} != {:?}",
                        c.id,
                        c.input,
                        lines,
                        a.key(o),
                        b.key(o)
                    ));
                }
            }
        }
        bad
    }
}

impl Report {
    pub fn total(&self) -> usize {
        self.total
    }

    /// Ids of the cases that pass.
    pub fn passing(&self) -> &BTreeSet<String> {
        &self.passing
    }

    /// The accuracy summary: whole-address and per-field counts.
    pub fn summary(&self) -> String {
        let total = self.total;
        let pct = |n: usize| {
            if total == 0 {
                0.0
            } else {
                100.0 * n as f64 / total as f64
            }
        };
        let mut s = format!("golden corpus: {total} cases\n");
        let _ = writeln!(
            s,
            "  whole address: {}/{total} ({:.1}%)",
            self.passing.len(),
            pct(self.passing.len())
        );
        for f in FIELDS {
            let n = self.field_ok.get(f).copied().unwrap_or(0);
            if OPTIONAL_FIELDS.contains(&f) {
                let of = self.field_n.get(f).copied().unwrap_or(0);
                let _ = writeln!(s, "  {f:<20} {n:>4}/{of} checked");
            } else {
                let _ = writeln!(s, "  {f:<20} {n:>4}/{total} ({:.1}%)", pct(n));
            }
        }
        s
    }

    /// The failing cases with their field differences, in id order.
    pub fn failure_details(&self) -> String {
        let all: Vec<_> = self.failures.values().map(String::as_str).collect();
        all.join("\n")
    }

    /// The field differences for one case, if it fails.
    pub fn failure(&self, id: &str) -> Option<&str> {
        self.failures.get(id).map(String::as_str)
    }

    /// Compare with a recorded baseline of passing ids.
    pub fn compare(&self, baseline: &BTreeSet<String>) -> Comparison {
        Comparison {
            stale: baseline.difference(&self.ids).cloned().collect(),
            regressed: baseline
                .intersection(&self.ids)
                .filter(|id| !self.passing.contains(*id))
                .cloned()
                .collect(),
            newly_passing: self.passing.difference(baseline).cloned().collect(),
        }
    }
}

/// The baseline file inside a corpus directory.
pub fn baseline_path(dir: &Path) -> PathBuf {
    dir.join("passing.txt")
}

/// Read `passing.txt`; a missing file is an empty baseline.
pub fn read_baseline(dir: &Path) -> BTreeSet<String> {
    fs::read_to_string(baseline_path(dir))
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(String::from)
        .collect()
}

/// Rewrite `passing.txt` from a report. Returns the path written.
pub fn write_baseline(dir: &Path, report: &Report) -> io::Result<PathBuf> {
    let mut out = String::from(
        "# Golden-corpus cases that pass. Regenerate with\n\
         # UPDATE_CORPUS_BASELINE=1 cargo test -p addr-core --test corpus\n\
         # or `addr corpus test --update-baseline`\n",
    );
    for id in &report.passing {
        let _ = writeln!(out, "{id}");
    }
    let path = baseline_path(dir);
    fs::write(&path, out)?;
    Ok(path)
}
