//! Place resolution (Milestone 5; `docs/design.md` section 13, decisions 13
//! and 14).
//!
//! Runs after assembly. Names the place the city stands for, from the place
//! lists compiled into [`Lexicons`] (the national list for every state, with
//! a curated state pack above it), or the ZIP's main place when there is no
//! city. [`ParsedAddress::city`] is never changed: the match is reported in
//! [`ParsedAddress::place`], beside it. A city the list knows whose ZIP the
//! list does not give it gets [`Warning::ZipCityMismatch`], but only in a state
//! with a curated pack: the national list names one place per ZIP, and many
//! ZIPs are used by several, so the warning would fire too often elsewhere
//! (design.md decision 30).

use crate::lexicon::{Lexicons, MatchKind, ZipPlaces};
use crate::types::{ParsedAddress, Place, PlaceMatch, PlaceSource, Warning};

pub(crate) fn resolve(addr: &mut ParsedAddress, lex: &Lexicons) {
    let zip = addr.zip5.as_deref().and_then(|z| lex.zip_places(z));
    // The state written on the address wins; with none, the ZIP's.
    let state = match (addr.state, zip) {
        (Some(s), _) => s.code(),
        (None, Some(z)) => z.main().state(),
        (None, None) => return,
    };
    let zip = zip.filter(|z| z.main().state() == state);
    let Some(city) = addr.city.as_deref() else {
        if addr.military_post_office.is_none() {
            addr.place = zip.map(|z| PlaceMatch {
                place: z.main(),
                source: PlaceSource::Zip,
            });
        }
        return;
    };
    let found = lookup(lex, state, city).or_else(|| {
        complete(lex, city, zip?).map(|place| PlaceMatch {
            place,
            source: PlaceSource::Completed,
        })
    });
    if let (Some(m), Some(z)) = (found, zip) {
        if lex.has_pack(state) && !z.contains(&m.place) {
            addr.diagnostics.warn(Warning::ZipCityMismatch);
        }
    }
    addr.place = found;
}

/// The city by name or listed spelling. A trailing copy of the state
/// (`BOCA RATON FL` in a city field) is ignored.
fn lookup(lex: &Lexicons, state: &str, city: &str) -> Option<PlaceMatch> {
    let hit = lex.place(state, city).or_else(|| {
        let (head, last) = city.rsplit_once(' ')?;
        let same = lex.state(last)?.value.code() == state;
        same.then(|| lex.place(state, head)).flatten()
    })?;
    let source = match hit.kind {
        MatchKind::Standard => PlaceSource::Name,
        _ => PlaceSource::Variant,
    };
    Some(PlaceMatch {
        place: hit.value,
        source,
    })
}

/// A city cut off at a field limit (`FERNANDINA B`, `WESLEY CHA`, `TAL`):
/// the one place for the ZIP whose words start with the city's. Every word
/// but the last must match in full (abbreviations aside); the last may be
/// cut. A single cut word needs three letters.
fn complete(lex: &Lexicons, city: &str, zip: ZipPlaces) -> Option<Place> {
    let words: Vec<&str> = city.split(' ').filter(|w| !w.is_empty()).collect();
    let (last, full) = words.split_last()?;
    if full.is_empty() && last.len() < 3 {
        return None;
    }
    let full: Vec<String> = full.iter().map(|w| lex.place_key(w)).collect();
    let mut found = None;
    for p in zip.all() {
        let names: Vec<&str> = p.name().split(' ').collect();
        let fits = names.len() > full.len()
            && full.iter().zip(&names).all(|(w, n)| *w == lex.place_key(n))
            && names[full.len()].starts_with(last);
        if fits {
            if found.is_some_and(|f| f != p) {
                return None;
            }
            found = Some(p);
        }
    }
    found
}

#[cfg(all(test, feature = "state-fl"))]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::{Fields, Parser, PlaceSource, Warning};

    fn place(s: &str) -> Option<(&'static str, PlaceSource)> {
        let a = Parser::new().parse(s).unwrap();
        a.place.map(|m| (m.place.name(), m.source))
    }

    #[test]
    fn by_name_and_variant() {
        assert_eq!(
            place("1 MAIN ST, W. PALM BCH, FL 33401"),
            Some(("WEST PALM BEACH", PlaceSource::Name))
        );
        assert_eq!(
            place("1 MAIN ST, BOCA RATOM, FL 33432"),
            Some(("BOCA RATON", PlaceSource::Variant))
        );
        let f = Fields {
            line1: Some("1 MAIN ST".into()),
            city: Some("BOCA RATON FL".into()),
            state: Some("FL".into()),
            zip: Some("33432".into()),
            ..Fields::default()
        };
        let a = Parser::new().parse_fields(&f).unwrap();
        assert_eq!(a.place.unwrap().place.name(), "BOCA RATON");
    }

    #[test]
    fn completed_and_from_zip() {
        assert_eq!(
            place("85231 BOSTWICK DR, FERNANDINA B, FL 32034"),
            Some(("FERNANDINA BEACH", PlaceSource::Completed))
        );
        assert_eq!(
            place("123 MAIN ST, FL 32507"),
            Some(("PENSACOLA", PlaceSource::Zip))
        );
        assert_eq!(
            place("WINGATE ST 32507"),
            Some(("PENSACOLA", PlaceSource::Zip))
        );
        // The national list covers a state with no pack; with no city and
        // no ZIP there is nothing to name.
        assert_eq!(
            place("1 MAIN ST, ATLANTA, GA 30301"),
            Some(("ATLANTA", PlaceSource::Name))
        );
        assert_eq!(
            place("1 MAIN ST, MIAMI, FL"),
            Some(("MIAMI", PlaceSource::Name))
        );
        assert_eq!(place("1 MAIN ST"), None);
    }

    #[test]
    fn mismatch() {
        let a = Parser::new().parse("1 MAIN ST, MIAMI, FL 32507").unwrap();
        assert!(a.diagnostics.has(&Warning::ZipCityMismatch));
        assert_eq!(a.city.as_deref(), Some("MIAMI"));
        let a = Parser::new()
            .parse("1 MAIN ST, AVENTURA, FL 33180")
            .unwrap();
        assert!(!a.diagnostics.has(&Warning::ZipCityMismatch));
        // Outside a curated-pack state the ZIP names one place only, so a
        // different city is not flagged (decision 30).
        let a = Parser::new()
            .parse("1 MAIN ST, ST LOUIS, MO 63301")
            .unwrap();
        assert!(!a.diagnostics.has(&Warning::ZipCityMismatch));
        assert_eq!(a.place.unwrap().place.name(), "SAINT LOUIS");
        // An unknown city is not a mismatch.
        let a = Parser::new()
            .parse("1 MAIN ST, NOWHEREVILLE, FL 33404")
            .unwrap();
        assert!(!a.diagnostics.has(&Warning::ZipCityMismatch));
        assert!(a.place.is_none());
    }
}
