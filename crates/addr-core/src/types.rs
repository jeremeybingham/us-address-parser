//! Core data types. Field names are kept aligned with the RESO emitter.
//!
//! Lexicon-backed values ([`StreetSuffix`], [`UnitType`], [`StateCode`]) can
//! only be obtained from [`crate::Lexicons`], so a value of one of these types
//! is always a real Pub 28 entry. They hold `&'static str`s into the
//! compiled tables: cheap to copy, and comparable by value.

use std::fmt;

use crate::VERSION;

/// Compass directional (N, S, E, W, NE, NW, SE, SW), used as a street
/// predirectional or postdirectional. Source: `data/directionals.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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

impl Directional {
    pub const ALL: [Directional; 8] = [
        Directional::N,
        Directional::S,
        Directional::E,
        Directional::W,
        Directional::NE,
        Directional::NW,
        Directional::SE,
        Directional::SW,
    ];

    /// Standard abbreviation, as emitted in USPS output: `"NE"`.
    pub const fn abbr(self) -> &'static str {
        match self {
            Directional::N => "N",
            Directional::S => "S",
            Directional::E => "E",
            Directional::W => "W",
            Directional::NE => "NE",
            Directional::NW => "NW",
            Directional::SE => "SE",
            Directional::SW => "SW",
        }
    }

    /// Full name: `"NORTHEAST"`.
    pub const fn name(self) -> &'static str {
        match self {
            Directional::N => "NORTH",
            Directional::S => "SOUTH",
            Directional::E => "EAST",
            Directional::W => "WEST",
            Directional::NE => "NORTHEAST",
            Directional::NW => "NORTHWEST",
            Directional::SE => "SOUTHEAST",
            Directional::SW => "SOUTHWEST",
        }
    }
}

impl fmt::Display for Directional {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.abbr())
    }
}

/// Street suffix (street type), e.g. AVE, BLVD. Source: Pub 28 Appendix C1,
/// `data/suffixes.toml`. Obtain one with [`crate::Lexicons::suffix`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StreetSuffix {
    pub(crate) abbr: &'static str,
    pub(crate) name: &'static str,
}

impl StreetSuffix {
    /// Postal Service standard abbreviation: `"AVE"`.
    pub fn abbr(&self) -> &'static str {
        self.abbr
    }

    /// Full suffix name: `"AVENUE"`.
    pub fn name(&self) -> &'static str {
        self.name
    }
}

impl fmt::Display for StreetSuffix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.abbr)
    }
}

/// Secondary unit designator, e.g. APT, STE. Source: Pub 28 Appendix C2,
/// `data/units.toml`. Obtain one with [`crate::Lexicons::unit`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnitType {
    pub(crate) abbr: &'static str,
    pub(crate) name: &'static str,
    pub(crate) range_required: bool,
}

impl UnitType {
    /// Standard abbreviation: `"STE"`.
    pub fn abbr(&self) -> &'static str {
        self.abbr
    }

    /// Full designator name: `"SUITE"`.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Whether Pub 28 expects a secondary range (number) after this
    /// designator. False for BSMT, FRNT, LBBY, LOWR, OFC, PH, REAR, SIDE, UPPR.
    pub fn range_required(&self) -> bool {
        self.range_required
    }
}

impl fmt::Display for UnitType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.abbr)
    }
}

/// What a [`StateCode`] stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StateKind {
    State,
    /// The District of Columbia.
    District,
    /// AS, GU, MP, PR, VI.
    Territory,
    /// FM, MH, PW.
    FreelyAssociated,
    /// Armed Forces codes AA, AE, AP (APO/FPO/DPO addresses).
    Military,
}

/// Two-letter USPS state or possession code. Source: Pub 28 Appendix B,
/// `data/states.toml`. Obtain one with [`crate::Lexicons::state`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StateCode {
    pub(crate) code: &'static str,
    pub(crate) name: &'static str,
    pub(crate) kind: StateKind,
}

impl StateCode {
    /// The two-letter code: `"FL"`.
    pub fn code(&self) -> &'static str {
        self.code
    }

    /// Full name: `"FLORIDA"`.
    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn kind(&self) -> StateKind {
        self.kind
    }
}

impl fmt::Display for StateCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code)
    }
}

/// Rural route or highway contract route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RouteKind {
    /// Rural route, emitted as `RR`.
    Rural,
    /// Highway contract route (formerly star route), emitted as `HC`.
    HighwayContract,
}

impl RouteKind {
    pub const fn abbr(self) -> &'static str {
        match self {
            RouteKind::Rural => "RR",
            RouteKind::HighwayContract => "HC",
        }
    }
}

/// Rural route or highway contract route plus box: `RR 2 BOX 152`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuralRoute {
    pub kind: RouteKind,
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

/// Non-fatal parse issues, for routing rows to review. Extensible; keep
/// `#[non_exhaustive]`. [`Warning::code`] gives a stable machine-readable
/// name for storing in a database column.
///
/// Several variants come from statuses that proved useful on real county
/// data in auction-tracker (see `docs/prior-art.md`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Warning {
    /// Some input could not be assigned to a component; see
    /// [`Diagnostics::unparsed`].
    UnparsedRemainder,
    /// A token could be read more than one way and the parser picked one.
    AmbiguousToken,
    /// The street line is stand-in text ("NO SITUS", "UNKNOWN"), not an
    /// address. Source list: `data/special.toml` `[placeholder]`.
    Placeholder,
    /// A street with no house number.
    MissingStreetNumber,
    /// House number 0, usually vacant land. Treated as no number.
    ZeroStreetNumber,
    /// The number runs into a numbered street, so the split is a guess:
    /// "409742ND SQ" is 4097 42ND or 40974 2ND.
    AmbiguousStreetNumber,
    /// Several addresses in one input ("5210/5212 BRISTO ST").
    MultipleAddresses,
    /// The input looks cut off at a field-length limit.
    PossiblyTruncated,
    /// A unit designator with nothing after it, for a designator that
    /// requires a number ("APT", "#").
    MissingUnitNumber,
    /// A recognized spelling that the Pub 28 tables do not list (e.g. "TRC"
    /// for TRACE). Normalized anyway.
    NonStandardAbbreviation,
    MissingCity,
    MissingState,
    /// A state-position token that is not a USPS code or known name.
    UnknownState,
    MissingZip,
    /// A ZIP that is not 5 digits, or ZIP+4 that is not 4 digits.
    InvalidZip,
    /// The parser pipeline is not implemented yet (skeleton only).
    NotImplemented,
}

impl Warning {
    /// Stable snake_case identifier. Part of the public contract: never
    /// change an existing code; add new ones.
    pub const fn code(&self) -> &'static str {
        match self {
            Warning::UnparsedRemainder => "unparsed_remainder",
            Warning::AmbiguousToken => "ambiguous_token",
            Warning::Placeholder => "placeholder",
            Warning::MissingStreetNumber => "missing_street_number",
            Warning::ZeroStreetNumber => "zero_street_number",
            Warning::AmbiguousStreetNumber => "ambiguous_street_number",
            Warning::MultipleAddresses => "multiple_addresses",
            Warning::PossiblyTruncated => "possibly_truncated",
            Warning::MissingUnitNumber => "missing_unit_number",
            Warning::NonStandardAbbreviation => "non_standard_abbreviation",
            Warning::MissingCity => "missing_city",
            Warning::MissingState => "missing_state",
            Warning::UnknownState => "unknown_state",
            Warning::MissingZip => "missing_zip",
            Warning::InvalidZip => "invalid_zip",
            Warning::NotImplemented => "not_implemented",
        }
    }
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
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

impl Diagnostics {
    /// Whether `w` was reported.
    pub fn has(&self, w: &Warning) -> bool {
        self.warnings.contains(w)
    }

    /// Record a warning once; repeats are ignored.
    pub(crate) fn warn(&mut self, w: Warning) {
        if !self.has(&w) {
            self.warnings.push(w);
        }
    }
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

/// A parsed address. Always returned, even for partial parses; check
/// [`ParsedAddress::diagnostics`].
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedAddress {
    /// "123", "123-125", "123 1/2", "12A".
    pub street_number: Option<String>,
    pub street_dir_prefix: Option<Directional>,
    pub street_name: Option<String>,
    pub street_suffix: Option<StreetSuffix>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directional_abbr_and_name() {
        assert_eq!(Directional::NE.abbr(), "NE");
        assert_eq!(Directional::SW.name(), "SOUTHWEST");
        assert_eq!(Directional::W.to_string(), "W");
    }

    #[test]
    fn warning_codes_are_unique() {
        let all = [
            Warning::UnparsedRemainder,
            Warning::AmbiguousToken,
            Warning::Placeholder,
            Warning::MissingStreetNumber,
            Warning::ZeroStreetNumber,
            Warning::AmbiguousStreetNumber,
            Warning::MultipleAddresses,
            Warning::PossiblyTruncated,
            Warning::MissingUnitNumber,
            Warning::NonStandardAbbreviation,
            Warning::MissingCity,
            Warning::MissingState,
            Warning::UnknownState,
            Warning::MissingZip,
            Warning::InvalidZip,
            Warning::NotImplemented,
        ];
        let mut codes: Vec<_> = all.iter().map(Warning::code).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), all.len());
    }

    #[test]
    fn diagnostics_warn_once() {
        let mut d = Diagnostics::default();
        d.warn(Warning::MissingZip);
        d.warn(Warning::MissingZip);
        assert_eq!(d.warnings, vec![Warning::MissingZip]);
        assert!(d.has(&Warning::MissingZip));
    }
}
