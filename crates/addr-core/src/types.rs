//! Core data types. Field names are kept aligned with the RESO emitter.
//!
//! Lexicon-backed values ([`StreetSuffix`], [`UnitType`], [`StateCode`]) can
//! only be obtained from [`crate::Lexicons`], so a value of one of these types
//! is always a real Pub 28 entry. They hold `&'static str`s into the
//! compiled tables: cheap to copy, and comparable by value.

use std::fmt;

use crate::input::Fields;
use crate::lexicon::HighwayPrefix;
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

/// A city, town or community from a state pack (`data/packs/`,
/// `docs/packs/`). Obtain one with [`crate::Lexicons::place`] or
/// [`crate::Lexicons::zip_places`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Place {
    pub(crate) state: &'static str,
    pub(crate) name: &'static str,
    pub(crate) county: &'static str,
    pub(crate) kind: PlaceKind,
}

impl Place {
    /// The two-letter state code: `"FL"`.
    pub fn state(&self) -> &'static str {
        self.state
    }

    /// The place's standard name, as the pack spells it: `"PORT SAINT LUCIE"`.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The county name, upper case: `"SAINT LUCIE"`.
    pub fn county(&self) -> &'static str {
        self.county
    }

    pub fn kind(&self) -> PlaceKind {
        self.kind
    }
}

impl fmt::Display for Place {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}

/// What kind of place a [`Place`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlaceKind {
    /// An incorporated city, town or village.
    Municipality,
    /// A community or postal name with no city government.
    Place,
}

/// How the parser found the place for an address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlaceSource {
    /// The city matches the place's name (case, punctuation and
    /// abbreviations aside: `W PALM BCH` is WEST PALM BEACH).
    Name,
    /// The city is a spelling the pack lists for the place, usually a typo
    /// (`BOCA RATOM`).
    Variant,
    /// The city looks cut off, and exactly one place for the ZIP starts that
    /// way (`WESLEY CHA` is WESLEY CHAPEL).
    Completed,
    /// There is no city; the place is the ZIP's main place.
    Zip,
}

impl PlaceSource {
    /// Stable snake_case identifier.
    pub const fn code(self) -> &'static str {
        match self {
            PlaceSource::Name => "name",
            PlaceSource::Variant => "variant",
            PlaceSource::Completed => "completed",
            PlaceSource::Zip => "zip",
        }
    }
}

/// The place found for an address and how it was found (`docs/design.md`
/// section 13, decision 13). [`ParsedAddress::city`] keeps what the input
/// said; this names the place it stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlaceMatch {
    pub place: Place,
    pub source: PlaceSource,
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

/// A street named after a highway, with the prefix kept apart from the route
/// number: `COUNTY ROAD 23`, `US HIGHWAY 1`, `FM 1960`. Source: Pub 28
/// section 237 and Appendix F, `data/special.toml` `[[highway]]`.
///
/// The route is the road's number, not a house number or unit. It may hold
/// letters (`60E`, `1080A`) or be letters only (`STATE ROUTE MM`).
///
/// A state named in the highway name is kept apart too, as its code
/// (Appendix F: `KENTUCKY STATE HIGHWAY 625` is `KY STATE HIGHWAY 625`). A
/// state followed only by a route gets the HIGHWAY prefix (`KENTUCKY 440` is
/// `KY HIGHWAY 440`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Highway {
    pub state: Option<StateCode>,
    pub prefix: HighwayPrefix,
    pub route: String,
}

impl Highway {
    /// The standard name: state code (if any), prefix and route, as in
    /// `KY STATE HIGHWAY 625` or `COUNTY ROAD 23`.
    pub fn name(&self) -> String {
        match self.state {
            Some(s) => format!("{} {} {}", s.code(), self.prefix.standard(), self.route),
            None => format!("{} {}", self.prefix.standard(), self.route),
        }
    }
}

/// Military delivery-line address type. Source: Pub 28 section 238.1,
/// `data/special.toml` `[[military]]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MilitaryKind {
    /// Consolidated Postal Room.
    Cpr,
    /// Official Postal Center.
    Opc,
    /// Postal Service Center.
    Psc,
    /// Unit Postal Room.
    Upr,
    Unit,
}

impl MilitaryKind {
    pub const fn abbr(self) -> &'static str {
        match self {
            MilitaryKind::Cpr => "CPR",
            MilitaryKind::Opc => "OPC",
            MilitaryKind::Psc => "PSC",
            MilitaryKind::Upr => "UPR",
            MilitaryKind::Unit => "UNIT",
        }
    }
}

/// Military or Department of State delivery line: `PSC 1650 BOX 10`,
/// `UNIT 9900 BOX 0500` (Pub 28 sections 238.1 and 239).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MilitaryBox {
    pub kind: MilitaryKind,
    pub number: String,
    pub box_number: Option<String>,
}

/// Military post office, written where the city goes, with AA, AE or AP as
/// the state: `APO AE 09001` (Pub 28 sections 225.1 and 239).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MilitaryPostOffice {
    Apo,
    Fpo,
    Dpo,
}

impl MilitaryPostOffice {
    pub const fn abbr(self) -> &'static str {
        match self {
            MilitaryPostOffice::Apo => "APO",
            MilitaryPostOffice::Fpo => "FPO",
            MilitaryPostOffice::Dpo => "DPO",
        }
    }
}

/// How the input was supplied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputKind {
    SingleLine,
    Fields,
}

/// One field of [`crate::Fields`] input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldName {
    Line1,
    Line2,
    City,
    State,
    Zip,
}

/// Byte range into the original input (decision 13), not into cleaned text.
///
/// For single-line input `field` is `None` and the range indexes
/// [`SourceInfo::original`]. For [`crate::Fields`] input it names the field
/// and the range indexes that field's string; see [`SourceInfo::text`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub field: Option<FieldName>,
    pub start: usize,
    pub end: usize,
}

/// A [`ParsedAddress`] component, for labeling spans.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Component {
    StreetNumber,
    StreetDirPrefix,
    StreetName,
    StreetSuffix,
    StreetDirSuffix,
    Highway,
    UnitType,
    UnitNumber,
    SecondUnitType,
    SecondUnitNumber,
    PoBox,
    RuralRoute,
    GeneralDelivery,
    Military,
    MilitaryPostOffice,
    Attention,
    Building,
    City,
    State,
    Zip5,
    Zip4,
    /// Text the parser could not place ([`Diagnostics::unparsed`]).
    Unparsed,
}

/// Where a component came from in the input. A component built from
/// separated pieces (an unparsed remainder) has one span per piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComponentSpan {
    pub component: Component,
    pub span: Span,
}

/// Original input and provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceInfo {
    /// The input string, or for [`crate::Fields`] input the non-empty fields
    /// joined with `", "` (for display; spans index the fields themselves).
    pub original: String,
    pub kind: InputKind,
    /// The fields as given, for [`InputKind::Fields`].
    pub fields: Option<Fields>,
    /// Component spans, in input order.
    pub spans: Vec<ComponentSpan>,
}

impl SourceInfo {
    /// The original text a span covers, or `None` if it does not index the
    /// input (which would be a parser bug).
    pub fn text(&self, span: &Span) -> Option<&str> {
        let s = match (span.field, &self.fields) {
            (None, _) => self.original.as_str(),
            (Some(f), Some(fields)) => fields.get(f)?,
            (Some(_), None) => return None,
        };
        s.get(span.start..span.end)
    }

    /// The spans recorded for one component.
    pub fn spans_of(&self, component: Component) -> impl Iterator<Item = &Span> {
        self.spans
            .iter()
            .filter(move |c| c.component == component)
            .map(|c| &c.span)
    }
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
    /// A house number and a unit with no street (`123 UNIT 1`).
    MissingStreetName,
    /// House number 0, usually vacant land. Treated as no number.
    ZeroStreetNumber,
    /// The number runs into a numbered street, so the split is a guess:
    /// "409742ND SQ" is 4097 42ND or 40974 2ND.
    AmbiguousStreetNumber,
    /// Several addresses in one input ("5210/5212 BRISTO ST").
    MultipleAddresses,
    /// A street address and a PO Box in one record (Pub 28 section 215).
    /// Both are kept; see [`ParsedAddress::po_box`].
    DualAddress,
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
}

impl Warning {
    /// Every variant, in declaration order. Add new variants here too; the
    /// golden-corpus runner checks expected warning codes against this list.
    pub const ALL: [Warning; 17] = [
        Warning::UnparsedRemainder,
        Warning::AmbiguousToken,
        Warning::Placeholder,
        Warning::MissingStreetNumber,
        Warning::MissingStreetName,
        Warning::ZeroStreetNumber,
        Warning::AmbiguousStreetNumber,
        Warning::MultipleAddresses,
        Warning::DualAddress,
        Warning::PossiblyTruncated,
        Warning::MissingUnitNumber,
        Warning::NonStandardAbbreviation,
        Warning::MissingCity,
        Warning::MissingState,
        Warning::UnknownState,
        Warning::MissingZip,
        Warning::InvalidZip,
    ];

    /// Stable snake_case identifier. Part of the public contract: never
    /// change an existing code; add new ones.
    pub const fn code(&self) -> &'static str {
        match self {
            Warning::UnparsedRemainder => "unparsed_remainder",
            Warning::AmbiguousToken => "ambiguous_token",
            Warning::Placeholder => "placeholder",
            Warning::MissingStreetNumber => "missing_street_number",
            Warning::MissingStreetName => "missing_street_name",
            Warning::ZeroStreetNumber => "zero_street_number",
            Warning::AmbiguousStreetNumber => "ambiguous_street_number",
            Warning::MultipleAddresses => "multiple_addresses",
            Warning::DualAddress => "dual_address",
            Warning::PossiblyTruncated => "possibly_truncated",
            Warning::MissingUnitNumber => "missing_unit_number",
            Warning::NonStandardAbbreviation => "non_standard_abbreviation",
            Warning::MissingCity => "missing_city",
            Warning::MissingState => "missing_state",
            Warning::UnknownState => "unknown_state",
            Warning::MissingZip => "missing_zip",
            Warning::InvalidZip => "invalid_zip",
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
    /// Planned: the calibrated probability (0.0 to 1.0) that the whole parse
    /// is right. `None` until calibrated (`docs/design.md` section 13,
    /// decision 12).
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
    /// The primary name. Empty for a highway address, whose name is in
    /// [`ParsedAddress::highway`].
    pub street_name: Option<String>,
    pub street_suffix: Option<StreetSuffix>,
    pub street_dir_suffix: Option<Directional>,
    /// Highway street name (`8932 HWY 2301`): prefix and route number.
    pub highway: Option<Highway>,
    /// Secondary unit designator. Empty for a bare unit (`DR 706`) or `#`.
    pub unit_type: Option<UnitType>,
    pub unit_number: Option<String>,
    /// A second unit after the first (`-UNIT B LT 18`): building then unit,
    /// unit then lot, and so on, in input order.
    pub second_unit_type: Option<UnitType>,
    pub second_unit_number: Option<String>,
    /// PO Box number. A record can hold a street address too (a dual
    /// address, Pub 28 section 215, with [`Warning::DualAddress`]): the key
    /// and RESO output use the street, the USPS output the box.
    pub po_box: Option<String>,
    pub rural_route: Option<RuralRoute>,
    /// The delivery line is GENERAL DELIVERY (Pub 28 section 261).
    pub general_delivery: bool,
    /// Military or Department of State box line (`PSC 1650 BOX 10`).
    pub military: Option<MilitaryBox>,
    /// APO, FPO or DPO in the city position. When set, `city` is empty.
    pub military_post_office: Option<MilitaryPostOffice>,
    /// An attention or care-of line, as written (`ATTN LAW DEPT`,
    /// `C/O HERITAGE MGMT LLC`). Pub 28 section 214 places it above the
    /// recipient, so it is not part of any emitted address.
    pub attention: Option<String>,
    /// A building name after the street or its unit (`EAST TOWER`,
    /// `ANNEX BUILDING`), as written. Not part of any emitted address.
    pub building: Option<String>,
    /// The city as the input wrote it.
    pub city: Option<String>,
    /// The place the city stands for, from the state packs, or the ZIP's
    /// main place when there is no city. `None` when no pack covers the
    /// state or nothing matched. Not part of any emitted address.
    pub place: Option<PlaceMatch>,
    pub state: Option<StateCode>,
    pub zip5: Option<String>,
    pub zip4: Option<String>,
    pub diagnostics: Diagnostics,
    pub source: SourceInfo,
}

impl ParsedAddress {
    /// Whether the record has a street address (a street name or highway).
    pub(crate) fn has_street(&self) -> bool {
        self.street_name.is_some() || self.highway.is_some()
    }

    /// Whether the record holds a street address and a PO Box (Pub 28
    /// section 215).
    pub(crate) fn is_dual(&self) -> bool {
        self.po_box.is_some() && self.has_street()
    }
}

impl ParsedAddress {
    pub(crate) fn empty(original: String, kind: InputKind, fields: Option<Fields>) -> Self {
        Self {
            street_number: None,
            street_dir_prefix: None,
            street_name: None,
            street_suffix: None,
            street_dir_suffix: None,
            highway: None,
            unit_type: None,
            unit_number: None,
            second_unit_type: None,
            second_unit_number: None,
            po_box: None,
            rural_route: None,
            general_delivery: false,
            military: None,
            military_post_office: None,
            attention: None,
            building: None,
            city: None,
            place: None,
            state: None,
            zip5: None,
            zip4: None,
            diagnostics: Diagnostics::default(),
            source: SourceInfo {
                original,
                kind,
                fields,
                spans: Vec::new(),
            },
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
        let all = Warning::ALL;
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
