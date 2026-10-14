//! Core data types. Field names are kept aligned with the RESO emitter.

use crate::VERSION;

/// Compass directional (N, S, E, W, NE, NW, SE, SW).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Directional {
    N,
    S,
    E,
    W,
    NE,
    NW,
    SE,
    SW,
}

/// Secondary unit designator. Populated from `data/units.toml` in Milestone 1.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnitType(pub String);

/// Two-letter USPS state code. Validated against `data/states.toml` in Milestone 1.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StateCode(pub String);

/// Rural route or highway contract route plus box.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuralRoute {
    /// `"RR"` or `"HC"`.
    pub kind: String,
    pub route: String,
    pub box_number: Option<String>,
}

/// How the input was supplied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputKind {
    SingleLine,
    Fields,
}

/// Original input and provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceInfo {
    pub original: String,
    pub kind: InputKind,
    // Component byte spans are added in Milestone 2.
}

/// Non-fatal parse issues. Extensible; keep `#[non_exhaustive]`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Warning {
    UnparsedRemainder,
    AmbiguousToken,
    MissingZip,
    /// The parser pipeline is not implemented yet (skeleton only).
    NotImplemented,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostics {
    pub warnings: Vec<Warning>,
    pub unparsed: Option<String>,
    /// Present only when the CRF tagger is used; semantics undefined until then.
    pub confidence: Option<f32>,
    pub library_version: &'static str,
    pub model_version: Option<&'static str>,
}

impl Default for Diagnostics {
    fn default() -> Self {
        Self {
            warnings: Vec::new(),
            unparsed: None,
            confidence: None,
            library_version: VERSION,
            model_version: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedAddress {
    pub street_number: Option<String>,
    pub street_dir_prefix: Option<Directional>,
    pub street_name: Option<String>,
    pub street_suffix: Option<String>,
    pub street_dir_suffix: Option<Directional>,
    pub unit_type: Option<UnitType>,
    pub unit_number: Option<String>,
    pub po_box: Option<String>,
    pub rural_route: Option<RuralRoute>,
    pub city: Option<String>,
    pub state: Option<StateCode>,
    pub zip5: Option<String>,
    pub zip4: Option<String>,
    pub diagnostics: Diagnostics,
    pub source: SourceInfo,
}

impl ParsedAddress {
    pub(crate) fn empty(original: String, kind: InputKind) -> Self {
        Self {
            street_number: None,
            street_dir_prefix: None,
            street_name: None,
            street_suffix: None,
            street_dir_suffix: None,
            unit_type: None,
            unit_number: None,
            po_box: None,
            rural_route: None,
            city: None,
            state: None,
            zip5: None,
            zip4: None,
            diagnostics: Diagnostics::default(),
            source: SourceInfo { original, kind },
        }
    }
}
