//! Lexicon lookups: street suffixes, directionals, secondary unit
//! designators, states, and special address forms.
//!
//! All lexicon access goes through [`Lexicons`]. The tables are compiled from
//! `data/*.toml` by `build.rs` into sorted static arrays, so lookups are a
//! key normalization plus a binary search, with no runtime data loading.
//!
//! `Lexicons` is the seam where state tuning packs will plug in later
//! (`docs/design.md` section 11): a pack adds spellings that resolve to the
//! same Pub 28 entries, so the value types keep pointing at the base tables.

mod key;
mod place_key;

pub use key::normalize_key;

use crate::types::{
    Directional, MilitaryKind, MilitaryPostOffice, Place, PlaceKind, StateCode, StateKind,
    StreetSuffix, UnitType,
};

/// How a looked-up spelling relates to its table entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MatchKind {
    /// The standard abbreviation or code itself ("AVE", "STE", "FL", "N").
    Standard,
    /// The full name ("AVENUE", "SUITE", "FLORIDA", "NORTH").
    FullName,
    /// Another spelling the source table lists ("AVNUE", "FLA" for states,
    /// "P O BOX").
    Variant,
    /// A spelling seen in real data that the Pub 28 table does not list
    /// ("TRC" for TRCE). Callers should report
    /// [`crate::Warning::NonStandardAbbreviation`].
    Observed,
}

/// A lexicon hit: the entry plus how the input spelled it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Lookup<T> {
    pub value: T,
    pub kind: MatchKind,
}

/// Non-street address words from `data/special.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Special {
    /// PO BOX (and synonyms such as DRAWER, CALLER).
    PoBox,
    /// The BOX word inside rural route and highway contract addresses.
    Box,
    /// Rural route: RR.
    RuralRoute,
    /// Highway contract route: HC.
    HighwayContract,
    /// GENERAL DELIVERY.
    GeneralDelivery,
    /// An attention or care-of word: ATTN, C/O.
    Attention,
}

impl Special {
    /// The standard form, as emitted.
    pub const fn standard(self) -> &'static str {
        match self {
            Special::PoBox => "PO BOX",
            Special::Box => "BOX",
            Special::RuralRoute => "RR",
            Special::HighwayContract => "HC",
            Special::GeneralDelivery => "GENERAL DELIVERY",
            Special::Attention => "ATTN",
        }
    }
}

/// A highway prefix such as COUNTY ROAD or US HIGHWAY; the route number that
/// follows it is part of the street name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HighwayPrefix {
    standard: &'static str,
}

impl HighwayPrefix {
    /// The standard form: `"COUNTY ROAD"`.
    pub fn standard(&self) -> &'static str {
        self.standard
    }
}

// Rows of the generated tables.
struct SuffixRow {
    name: &'static str,
    abbr: &'static str,
}

struct UnitRow {
    name: &'static str,
    abbr: &'static str,
    range_required: bool,
}

struct StateRow {
    code: &'static str,
    name: &'static str,
    kind: StateKind,
}

struct PlaceRow {
    state: &'static str,
    name: &'static str,
    county: &'static str,
    kind: PlaceKind,
}

mod tables {
    #[allow(unused_imports)] // PlaceKind is unused when no pack is enabled.
    use super::{MatchKind, PlaceKind, PlaceRow, Special, StateKind, StateRow, SuffixRow, UnitRow};
    use crate::types::{Directional, MilitaryKind, MilitaryPostOffice};

    include!(concat!(env!("OUT_DIR"), "/lexicon_tables.rs"));
}

pub use tables::{MAX_FORM_WORDS, MAX_PLACE_WORDS};

fn place_row(i: u32) -> Option<Place> {
    let r = tables::PLACES.get(usize::try_from(i).ok()?)?;
    Some(Place {
        state: r.state,
        name: r.name,
        county: r.county,
        kind: r.kind,
    })
}

/// The places a ZIP belongs to, from the state packs: its main place, and
/// any place it shares with (the ZIP straddles a boundary).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZipPlaces {
    main: Place,
    shared: &'static [u32],
}

impl ZipPlaces {
    /// The ZIP's main place (its GeoNames place name).
    pub fn main(&self) -> Place {
        self.main
    }

    /// Other places that use the ZIP.
    pub fn shared(&self) -> impl Iterator<Item = Place> + '_ {
        self.shared.iter().filter_map(|&i| place_row(i))
    }

    /// The main place, then the shared ones.
    pub fn all(&self) -> impl Iterator<Item = Place> + '_ {
        std::iter::once(self.main).chain(self.shared())
    }

    /// Whether `p` uses this ZIP.
    pub fn contains(&self, p: &Place) -> bool {
        self.all().any(|q| q == *p)
    }
}

fn find<'a, V>(index: &'a [(&'static str, V)], key: &str) -> Option<&'a V> {
    index
        .binary_search_by(|(k, _)| (*k).cmp(key))
        .ok()
        .and_then(|i| index.get(i))
        .map(|(_, v)| v)
}

/// Access to the compiled lexicon tables. Cheap to copy, `Send + Sync`.
///
/// Every lookup normalizes its input with [`normalize_key`] first, so
/// `"Ave."`, `"ave"` and `"AVE"` all match. Multi-word forms ("NORTH EAST",
/// "NEW YORK", "PO BOX") match when passed as one space-separated string; no
/// form has more than [`MAX_FORM_WORDS`] words.
#[derive(Debug, Clone, Copy, Default)]
pub struct Lexicons {
    _private: (),
}

impl Lexicons {
    /// The built-in tables (Pub 28 base lexicon).
    pub const fn builtin() -> Self {
        Lexicons { _private: () }
    }

    /// Street suffix: `"AVENUE"`, `"Ave."`, `"AVNUE"` -> AVE.
    pub fn suffix(&self, s: &str) -> Option<Lookup<StreetSuffix>> {
        let (i, kind) = *find(tables::SUFFIX_INDEX, &normalize_key(s))?;
        let row = tables::SUFFIXES.get(usize::from(i))?;
        Some(Lookup {
            value: StreetSuffix {
                abbr: row.abbr,
                name: row.name,
            },
            kind,
        })
    }

    /// Directional: `"NORTH EAST"`, `"N.E."`, `"ne"` -> NE.
    pub fn directional(&self, s: &str) -> Option<Lookup<Directional>> {
        let (value, kind) = *find(tables::DIRECTIONAL_INDEX, &normalize_key(s))?;
        Some(Lookup { value, kind })
    }

    /// Secondary unit designator: `"Suite"` -> STE. Does not match `"#"`,
    /// which stands in for an unknown designator and is handled by the tagger.
    pub fn unit(&self, s: &str) -> Option<Lookup<UnitType>> {
        let (i, kind) = *find(tables::UNIT_INDEX, &normalize_key(s))?;
        let row = tables::UNITS.get(usize::from(i))?;
        Some(Lookup {
            value: UnitType {
                abbr: row.abbr,
                name: row.name,
                range_required: row.range_required,
            },
            kind,
        })
    }

    /// State or possession, by code, full name or traditional abbreviation:
    /// `"fl"`, `"Florida"`, `"Fla."` -> FL.
    pub fn state(&self, s: &str) -> Option<Lookup<StateCode>> {
        let (i, kind) = *find(tables::STATE_INDEX, &normalize_key(s))?;
        let row = tables::STATES.get(usize::from(i))?;
        Some(Lookup {
            value: StateCode {
                code: row.code,
                name: row.name,
                kind: row.kind,
            },
            kind,
        })
    }

    /// Special address word: `"P.O. Box"` -> PO BOX, `"Rural Route"` -> RR.
    pub fn special(&self, s: &str) -> Option<Lookup<Special>> {
        let (value, kind) = *find(tables::SPECIAL_INDEX, &normalize_key(s))?;
        Some(Lookup { value, kind })
    }

    /// Highway prefix: `"County Rd"` -> COUNTY ROAD, `"HWY"` -> HIGHWAY.
    pub fn highway(&self, s: &str) -> Option<Lookup<HighwayPrefix>> {
        let (i, kind) = *find(tables::HIGHWAY_INDEX, &normalize_key(s))?;
        let standard = *tables::HIGHWAYS.get(usize::from(i))?;
        Some(Lookup {
            value: HighwayPrefix { standard },
            kind,
        })
    }

    /// Military delivery-line type: `"PSC"`, `"Postal Service Center"` -> PSC.
    pub fn military(&self, s: &str) -> Option<Lookup<MilitaryKind>> {
        let (value, kind) = *find(tables::MILITARY_INDEX, &normalize_key(s))?;
        Some(Lookup { value, kind })
    }

    /// Military post office: `"APO"`, `"FPO"`, `"DPO"`.
    pub fn military_post_office(&self, s: &str) -> Option<Lookup<MilitaryPostOffice>> {
        let (value, kind) = *find(tables::MILITARY_PO_INDEX, &normalize_key(s))?;
        Some(Lookup { value, kind })
    }

    /// Whether a whole street line is placeholder text ("NO SITUS",
    /// "0 UNKNOWN", "00 UNASSIGNED LOCATION RE"), or has no letters or digits
    /// at all ("-"). A leading run of zeros is ignored, and an
    /// "UNASSIGNED LOCATION" line may end in a short trailing code.
    /// A word that ends a building name (`TOWER` in `EAST TOWER`).
    /// Source: `data/special.toml` `[building]`.
    pub fn is_building_word(&self, s: &str) -> bool {
        find_str(tables::BUILDING_WORDS, &normalize_key(s))
    }

    pub fn is_placeholder(&self, s: &str) -> bool {
        let key = normalize_key(s);
        if !key.chars().any(char::is_alphanumeric) {
            return true;
        }
        let rest = match key.split_once(' ') {
            Some((zeros, rest)) if zeros.bytes().all(|b| b == b'0') => rest,
            _ => key.as_str(),
        };
        if find_str(tables::PLACEHOLDERS, rest) {
            return true;
        }
        // "UNASSIGNED LOCATION RE": a trailing code of up to two letters.
        match rest.rsplit_once(' ') {
            Some((head, code)) => {
                head == "UNASSIGNED LOCATION"
                    && code.len() <= 2
                    && code.bytes().all(|b| b.is_ascii_alphabetic())
            }
            None => false,
        }
    }

    /// The place-name comparison key: case, punctuation, spacing and
    /// abbreviations do not count (`W. Palm Bch` and `WEST PALM BEACH`
    /// share one). `ST`, `FT`, `MT` stand for SAINT, FORT, MOUNT; other
    /// words are expanded through the directional and suffix tables.
    pub fn place_key(&self, s: &str) -> String {
        place_key::place_key(s, |w| {
            self.directional(w)
                .map(|l| l.value.name().to_string())
                .or_else(|| self.suffix(w).map(|l| l.value.name().to_string()))
        })
    }

    /// Whether a state pack for `state` (a USPS code) is compiled in.
    pub fn has_pack(&self, state: &str) -> bool {
        tables::PACK_STATES.contains(&state)
    }

    /// A place in `state` (a USPS code) by name or a listed spelling:
    /// `"W Palm Bch"` -> WEST PALM BEACH ([`MatchKind::Standard`]),
    /// `"BOCA RATOM"` -> BOCA RATON ([`MatchKind::Variant`]). `None` when
    /// no compiled-in pack lists it.
    pub fn place(&self, state: &str, s: &str) -> Option<Lookup<Place>> {
        let key = format!("{state}|{}", self.place_key(s));
        let (i, kind) = *find(tables::PLACE_INDEX, &key)?;
        Some(Lookup {
            value: place_row(i)?,
            kind,
        })
    }

    /// Whether `s` names a place (by name, not a listed misspelling) in any
    /// compiled-in pack.
    pub fn is_place(&self, s: &str) -> bool {
        tables::PACK_STATES.iter().any(|st| {
            self.place(st, s)
                .is_some_and(|l| l.kind == MatchKind::Standard)
        })
    }

    /// The places a 5-digit ZIP belongs to, from the compiled-in packs.
    pub fn zip_places(&self, zip5: &str) -> Option<ZipPlaces> {
        let (main, shared) = *find(tables::ZIP_INDEX, zip5)?;
        Some(ZipPlaces {
            main: place_row(main)?,
            shared,
        })
    }

    /// All places in the compiled-in packs, in table order.
    pub fn places(&self) -> impl Iterator<Item = Place> {
        (0..tables::PLACES.len()).filter_map(|i| place_row(u32::try_from(i).ok()?))
    }

    /// All street suffixes, in table order.
    pub fn suffixes(&self) -> impl Iterator<Item = StreetSuffix> {
        tables::SUFFIXES.iter().map(|r| StreetSuffix {
            abbr: r.abbr,
            name: r.name,
        })
    }

    /// All secondary unit designators, in table order.
    pub fn units(&self) -> impl Iterator<Item = UnitType> {
        tables::UNITS.iter().map(|r| UnitType {
            abbr: r.abbr,
            name: r.name,
            range_required: r.range_required,
        })
    }

    /// All states and possessions, in table order.
    pub fn states(&self) -> impl Iterator<Item = StateCode> {
        tables::STATES.iter().map(|r| StateCode {
            code: r.code,
            name: r.name,
            kind: r.kind,
        })
    }
}

fn find_str(sorted: &[&str], key: &str) -> bool {
    sorted.binary_search(&key).is_ok()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const LEX: Lexicons = Lexicons::builtin();

    fn suffix(s: &str) -> (&'static str, MatchKind) {
        let l = LEX.suffix(s).unwrap();
        (l.value.abbr(), l.kind)
    }

    #[test]
    fn indexes_are_sorted_and_unique() {
        fn check<V>(name: &str, index: &[(&str, V)]) {
            for w in index.windows(2) {
                assert!(w[0].0 < w[1].0, "{name}: {} !< {}", w[0].0, w[1].0);
            }
        }
        check("suffix", tables::SUFFIX_INDEX);
        check("directional", tables::DIRECTIONAL_INDEX);
        check("unit", tables::UNIT_INDEX);
        check("state", tables::STATE_INDEX);
        check("special", tables::SPECIAL_INDEX);
        check("highway", tables::HIGHWAY_INDEX);
        check("military", tables::MILITARY_INDEX);
        check("military_po", tables::MILITARY_PO_INDEX);
        check("place", tables::PLACE_INDEX);
        check("zip", tables::ZIP_INDEX);
        assert!(tables::PLACEHOLDERS.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn table_sizes() {
        assert_eq!(LEX.suffixes().count(), 202);
        assert_eq!(LEX.units().count(), 25); // Appendix C2 plus PMB (section 285)
        assert_eq!(LEX.states().count(), 62);
        assert_eq!(MAX_FORM_WORDS, 4); // FEDERATED STATES OF MICRONESIA
    }

    #[test]
    fn suffix_lookup() {
        assert_eq!(suffix("AVE"), ("AVE", MatchKind::Standard));
        assert_eq!(suffix("avenue"), ("AVE", MatchKind::FullName));
        assert_eq!(suffix("Avnue."), ("AVE", MatchKind::Variant));
        assert_eq!(suffix("TERR"), ("TER", MatchKind::Variant));
        assert_eq!(suffix("WY"), ("WAY", MatchKind::Variant));
        assert_eq!(suffix("TRC"), ("TRCE", MatchKind::Observed));
        assert_eq!(suffix("CRT"), ("CT", MatchKind::Observed));
        assert_eq!(suffix("CAUSWA"), ("CSWY", MatchKind::Variant));
        assert_eq!(suffix("COMMONS"), ("CMNS", MatchKind::FullName));
        // Plural primaries share an abbreviation.
        assert_eq!(suffix("PARKS"), ("PARK", MatchKind::Variant));
        assert_eq!(suffix("PARKWAYS"), ("PKWY", MatchKind::Variant));
        assert_eq!(LEX.suffix("CIRCLE").unwrap().value.name(), "CIRCLE");
        assert!(LEX.suffix("MAIN").is_none());
        assert!(LEX.suffix("").is_none());
    }

    #[test]
    fn every_suffix_abbr_and_name_round_trips() {
        for s in LEX.suffixes() {
            assert_eq!(LEX.suffix(s.abbr()).unwrap().value, s);
            assert_eq!(LEX.suffix(s.name()).unwrap().value, s);
        }
    }

    #[test]
    fn directional_lookup() {
        let d = |s| LEX.directional(s).map(|l| (l.value, l.kind));
        assert_eq!(d("N"), Some((Directional::N, MatchKind::Standard)));
        assert_eq!(d("north"), Some((Directional::N, MatchKind::FullName)));
        assert_eq!(d("N.E."), Some((Directional::NE, MatchKind::Standard)));
        assert_eq!(d("South West"), Some((Directional::SW, MatchKind::Variant)));
        assert_eq!(d("NORTH-WEST"), Some((Directional::NW, MatchKind::Variant)));
        assert_eq!(d("NORTE"), None);
        for dir in Directional::ALL {
            assert_eq!(d(dir.abbr()).unwrap().0, dir);
            assert_eq!(d(dir.name()).unwrap().0, dir);
        }
    }

    #[test]
    fn unit_lookup() {
        let ste = LEX.unit("Suite").unwrap();
        assert_eq!((ste.value.abbr(), ste.kind), ("STE", MatchKind::FullName));
        assert!(ste.value.range_required());
        assert!(!LEX.unit("PH").unwrap().value.range_required());
        assert_eq!(LEX.unit("HANGER").unwrap().value.abbr(), "HNGR");
        let hangar = LEX.unit("HANGAR").unwrap();
        assert_eq!(
            (hangar.value.abbr(), hangar.kind),
            ("HNGR", MatchKind::Observed)
        );
        assert_eq!(LEX.unit("UPPER").unwrap().value.abbr(), "UPPR");
        let un = LEX.unit("UN").unwrap();
        assert_eq!((un.value.abbr(), un.kind), ("UNIT", MatchKind::Observed));
        assert!(LEX.unit("#").is_none());
        for u in LEX.units() {
            assert_eq!(LEX.unit(u.abbr()).unwrap().value, u);
        }
        let no_range: Vec<_> = LEX
            .units()
            .filter(|u| !u.range_required())
            .map(|u| u.abbr())
            .collect();
        assert_eq!(
            no_range,
            ["BSMT", "FRNT", "LBBY", "LOWR", "OFC", "PH", "REAR", "SIDE", "UPPR"]
        );
    }

    #[test]
    fn state_lookup() {
        let fl = LEX.state("fl").unwrap();
        assert_eq!((fl.value.code(), fl.kind), ("FL", MatchKind::Standard));
        assert_eq!(fl.value.name(), "FLORIDA");
        assert_eq!(fl.value.kind(), StateKind::State);
        assert_eq!(LEX.state("Florida").unwrap().kind, MatchKind::FullName);
        assert_eq!(LEX.state("Fla.").unwrap().kind, MatchKind::Variant);
        assert_eq!(LEX.state("new  york").unwrap().value.code(), "NY");
        assert_eq!(LEX.state("PR").unwrap().value.kind(), StateKind::Territory);
        assert_eq!(LEX.state("AE").unwrap().value.kind(), StateKind::Military);
        assert_eq!(LEX.state("D.C.").unwrap().value.code(), "DC");
        assert!(LEX.state("ZZ").is_none());
        for s in LEX.states() {
            assert_eq!(LEX.state(s.code()).unwrap().value, s);
            assert_eq!(LEX.state(s.name()).unwrap().value, s);
        }
    }

    #[test]
    fn special_lookup() {
        let sp = |s| LEX.special(s).map(|l| (l.value, l.kind));
        assert_eq!(sp("PO BOX"), Some((Special::PoBox, MatchKind::Standard)));
        assert_eq!(sp("P.O. Box"), Some((Special::PoBox, MatchKind::Standard)));
        assert_eq!(sp("p o box"), Some((Special::PoBox, MatchKind::Variant)));
        assert_eq!(sp("POBOX"), Some((Special::PoBox, MatchKind::Variant)));
        assert_eq!(sp("Drawer"), Some((Special::PoBox, MatchKind::Variant)));
        // Pub 28 section 283.
        assert_eq!(sp("BIN"), Some((Special::PoBox, MatchKind::Variant)));
        assert_eq!(
            sp("Rural Route"),
            Some((Special::RuralRoute, MatchKind::Variant))
        );
        // Pub 28 section 244: RFD and RD become RR.
        assert_eq!(sp("RFD"), Some((Special::RuralRoute, MatchKind::Variant)));
        assert_eq!(sp("RD"), Some((Special::RuralRoute, MatchKind::Variant)));
        // Pub 28 section 251: these are input only; HC is the standard.
        assert_eq!(
            sp("Highway Contract Route"),
            Some((Special::HighwayContract, MatchKind::Variant))
        );
        assert_eq!(Special::HighwayContract.standard(), "HC");
        assert_eq!(
            sp("STAR ROUTE"),
            Some((Special::HighwayContract, MatchKind::Variant))
        );
        assert_eq!(
            sp("Gen Del"),
            Some((Special::GeneralDelivery, MatchKind::Variant))
        );
        assert_eq!(sp("BOX").unwrap().0, Special::Box);
        assert_eq!(Special::PoBox.standard(), "PO BOX");
        assert!(sp("MAIN").is_none());
    }

    #[test]
    fn highway_lookup() {
        let h = |s| LEX.highway(s).map(|l| l.value.standard());
        assert_eq!(h("County Rd"), Some("COUNTY ROAD"));
        assert_eq!(h("CR"), Some("COUNTY ROAD"));
        assert_eq!(h("HWY"), Some("HIGHWAY"));
        assert_eq!(h("US HWY"), Some("US HIGHWAY"));
        assert_eq!(h("farm to market"), Some("FM"));
        // Pub 28 Appendix F.
        assert_eq!(h("ST HIGHWAY"), Some("STATE HIGHWAY"));
        assert_eq!(h("ST RD"), Some("STATE ROAD"));
        assert_eq!(h("ST RT"), Some("STATE ROUTE"));
        assert_eq!(h("TSR"), Some("TOWNSHIP ROAD"));
        assert_eq!(h("IH"), Some("INTERSTATE"));
        assert_eq!(h("RTE"), Some("ROUTE"));
        assert_eq!(h("RD"), Some("ROAD"));
        assert_eq!(h("LOOP"), Some("LOOP"));
        // State-dependent; left to state packs.
        assert_eq!(h("SR"), None);
    }

    #[test]
    fn military_lookup() {
        assert_eq!(LEX.military("psc").unwrap().value, MilitaryKind::Psc);
        assert_eq!(
            LEX.military("Unit Postal Room").unwrap().value,
            MilitaryKind::Upr
        );
        assert_eq!(LEX.military("UNIT").unwrap().value, MilitaryKind::Unit);
        assert_eq!(
            LEX.military_post_office("fpo").unwrap().value,
            MilitaryPostOffice::Fpo
        );
        assert!(LEX.military("CMR").is_none());
    }

    #[test]
    fn placeholders() {
        for s in [
            "NO SITUS",
            "not assigned",
            "UNKNOWN",
            "0 UNKNOWN",
            "N/A",
            "PROPERTY ADDRESS UNKNOWN",
            "NO ADDRESS ON TAX ROLL",
            "00 UNASSIGNED LOCATION RE",
            "Unassigned Location PP",
            "PROPERTY LOCATION IS NOT AVAIL",
            "-",
            "",
        ] {
            assert!(LEX.is_placeholder(s), "{s:?}");
        }
        for s in [
            "123 MAIN ST",
            "UNKNOWN RD",
            "0 HIDDEN MEADOW CT",
            "ACCESS UNDETERMINED",
        ] {
            assert!(!LEX.is_placeholder(s), "{s:?}");
        }
    }

    #[test]
    fn place_key_rules() {
        let k = |s| LEX.place_key(s);
        assert_eq!(k("St. Petersburg"), "SAINTPETERSBURG");
        assert_eq!(k("W PALM BCH"), k("WEST PALM BEACH"));
        assert_eq!(k("ALTAMONTE SPGS"), k("ALTAMONTE SPRINGS"));
        assert_eq!(k("N FT MYERS"), k("NORTH FORT MYERS"));
        assert_eq!(k("DE FUNIAK SPRINGS"), k("DEFUNIAK SPRINGS"));
        assert_eq!(k("OPA-LOCKA"), k("OPA LOCKA"));
        assert_eq!(k("LAND O' LAKES"), k("LAND O LAKES"));
        assert_ne!(k("PALM BEACH"), k("PALM BEACH GARDENS"));
    }

    #[cfg(feature = "state-fl")]
    #[test]
    fn place_lookup() {
        let p = |s| LEX.place("FL", s).map(|l| (l.value.name(), l.kind));
        assert_eq!(
            p("W. Palm Bch"),
            Some(("WEST PALM BEACH", MatchKind::Standard))
        );
        assert_eq!(
            p("PORT ST LUCIE"),
            Some(("PORT SAINT LUCIE", MatchKind::Standard))
        );
        assert_eq!(p("BOCA RATOM"), Some(("BOCA RATON", MatchKind::Variant)));
        assert_eq!(p("WINGATE ST"), None);
        assert_eq!(LEX.place("GA", "MIAMI"), None);
        let miami = LEX.place("FL", "MIAMI").unwrap().value;
        assert_eq!(
            (miami.state(), miami.county(), miami.kind()),
            ("FL", "DADE", PlaceKind::Municipality)
        );
        assert!(LEX.has_pack("FL"));
        assert!(!LEX.has_pack("GA"));
        assert_eq!(LEX.places().count(), 665);
    }

    #[cfg(feature = "state-fl")]
    #[test]
    fn zip_lookup() {
        let z = LEX.zip_places("32507").unwrap();
        assert_eq!(z.main().name(), "PENSACOLA");
        assert!(z.contains(&z.main()));
        // A ZIP shared by two places.
        let aventura = LEX.place("FL", "AVENTURA").unwrap().value;
        let shared = LEX.zip_places("33180").unwrap();
        assert!(shared.contains(&aventura));
        assert_ne!(shared.main(), aventura);
        assert!(LEX.zip_places("10001").is_none());
        // Every place is found by its own name.
        for p in LEX.places() {
            assert_eq!(LEX.place(p.state(), p.name()).unwrap().value, p);
        }
    }

    #[test]
    fn normalize_key_rules() {
        assert_eq!(normalize_key("  p.o.   box "), "PO BOX");
        assert_eq!(normalize_key("N.E."), "NE");
        assert_eq!(normalize_key("north-west"), "NORTH-WEST");
        assert_eq!(normalize_key("\tNew\nYork"), "NEW YORK");
        assert_eq!(normalize_key("..."), "");
    }
}
