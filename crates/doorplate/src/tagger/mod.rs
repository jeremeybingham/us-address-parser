//! Rule-based tagger (pipeline step 4).
//!
//! Gives every token a [`Tag`]. Rules live in [`rules`]; this module holds
//! the types, the entry points and the checks that run once everything is
//! tagged. Following Pub 28 section 231, the last line (ZIP, state, city) is
//! read from the right first, then the delivery line from both ends inward.
//!
//! The tagger never guesses: a token no rule claims becomes
//! [`Tag::Unparsed`] and is reported in `Diagnostics::unparsed`.

mod rules;

use std::ops::Range;

use crate::lexicon::{Lexicons, ZipPlaces};
use crate::tokenize::Token;
use crate::types::{FieldName, Warning};

/// Lines at least this long that end in something incomplete (a cut-off
/// ZIP, a designator with no number, a cut-off word) get
/// [`Warning::PossiblyTruncated`]. County sites cut address lines at about
/// 30 characters.
pub(crate) const TRUNCATION_MIN_LEN: usize = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tag {
    /// No rule claimed the token yet. Becomes `Unparsed` when tagging ends.
    Untagged,
    /// Consumed without a value: `#`, separators, BOX words, placeholder
    /// text, a repeated copy of the city.
    Skip,
    Unparsed,
    StreetNumber,
    PreDir,
    Name,
    Suffix,
    PostDir,
    /// A state named before a highway prefix or route (Appendix F).
    HighwayState,
    HighwayPrefix,
    HighwayRoute,
    /// Unit designator; 0 for the first unit, 1 for the second.
    UnitType(u8),
    UnitNumber(u8),
    PoBox,
    PoBoxNumber,
    /// An attention or care-of line (Pub 28 section 214).
    Attention,
    /// A building name (`EAST TOWER`).
    Building,
    /// RR or HC word.
    Route,
    RouteNumber,
    BoxNumber,
    GeneralDelivery,
    MilitaryKind,
    MilitaryNumber,
    MilitaryBoxNumber,
    MilitaryPostOffice,
    City,
    State,
    Zip,
}

/// An uncertain rule that settled a boundary. Warnings report what went
/// wrong; these record what the tagger had to judge on a parse that may have
/// none. Kept as the input for a possible future confidence score.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Decision {
    /// The city is a line or comma segment of its own (`..., MIAMI`).
    CityByComma,
    /// One line with no commas: the city starts after the street's suffix
    /// (`12 LAKE SHORE DR CHICAGO`), or is all there is (`TAMPA 33616`).
    CityBySuffix,
    /// The city was found in the place list (`MAIN ST WEST PALM BEACH`).
    CityByPlace,
    /// A state with no ZIP after it (`..., MIAMI FL`).
    StateWithoutZip,
    /// A unit with no designator: after the suffix (`DR 706`, `DR B`) or a
    /// later line that is only a number.
    BareUnit,
    /// A single letter joined to the house number (`428 B SOUTH MAIN ST`).
    NumberLetter,
    /// The parser's `default_state` decided a tag (a city or a pack's
    /// highway spelling).
    DefaultState,
}

impl Decision {
    /// Stable snake_case identifier, read by the calibration tools.
    #[cfg(feature = "corpus")]
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Decision::CityByComma => "city_by_comma",
            Decision::CityBySuffix => "city_by_suffix",
            Decision::CityByPlace => "city_by_place",
            Decision::StateWithoutZip => "state_without_zip",
            Decision::BareUnit => "bare_unit",
            Decision::NumberLetter => "number_letter",
            Decision::DefaultState => "default_state",
        }
    }
}

pub(crate) struct Tagged {
    pub tags: Vec<Tag>,
    pub warnings: Vec<Warning>,
    /// Each uncertain decision once, in the order first made.
    pub decisions: Vec<Decision>,
}

/// Lengths used for the truncation check.
pub(crate) struct LineLengths {
    /// Character length of each segment, indexed by segment number.
    pub seg: Vec<usize>,
    /// Character length of the whole input (single-line input only).
    pub whole: Option<usize>,
}

pub(crate) struct Tagger<'a> {
    toks: &'a [Token],
    tags: Vec<Tag>,
    warnings: Vec<Warning>,
    decisions: Vec<Decision>,
    lex: Lexicons,
    /// A token where the input looks cut off, if any.
    incomplete: Option<usize>,
    zero_number: bool,
    lens: LineLengths,
    /// The state pack that applies, once the last line's state or ZIP is
    /// known.
    places: Option<PlaceCtx>,
    /// The parser's `default_state`, used when the input names no state.
    hint: Option<&'static str>,
    /// Whether a place found under the hint decided a tag.
    hint_used: std::cell::Cell<bool>,
}

/// The state whose pack is used to find the city, and the ZIP's places.
#[derive(Clone, Copy)]
pub(crate) struct PlaceCtx {
    state: &'static str,
    zip: Option<ZipPlaces>,
    /// The state is the parser's `default_state`, not the input's.
    hinted: bool,
}

impl<'a> Tagger<'a> {
    fn new(
        toks: &'a [Token],
        lex: Lexicons,
        lens: LineLengths,
        hint: Option<&'static str>,
    ) -> Self {
        Self {
            toks,
            tags: vec![Tag::Untagged; toks.len()],
            warnings: Vec::new(),
            decisions: Vec::new(),
            lex,
            incomplete: None,
            zero_number: false,
            lens,
            places: None,
            hint,
            hint_used: std::cell::Cell::new(false),
        }
    }

    fn warn(&mut self, w: Warning) {
        if !self.warnings.contains(&w) {
            self.warnings.push(w);
        }
    }

    fn decide(&mut self, d: Decision) {
        if !self.decisions.contains(&d) {
            self.decisions.push(d);
        }
    }

    fn set(&mut self, r: Range<usize>, tag: Tag) {
        for t in self.tags.get_mut(r).into_iter().flatten() {
            *t = tag;
        }
    }

    fn text(&self, i: usize) -> &'a str {
        self.toks.get(i).map_or("", |t| t.text.as_str())
    }

    /// First token index of the segment that token `i` is in.
    fn seg_start(&self, i: usize) -> usize {
        let seg = self.toks[i].seg;
        let mut s = i;
        while s > 0 && self.toks[s - 1].seg == seg {
            s -= 1;
        }
        s
    }

    fn finish(mut self) -> Tagged {
        for t in &mut self.tags {
            if *t == Tag::Untagged {
                *t = Tag::Unparsed;
            }
        }
        let has = |tags: &[Tag], tag: Tag| tags.contains(&tag);
        let street_like = has(&self.tags, Tag::Name)
            || has(&self.tags, Tag::Suffix)
            || has(&self.tags, Tag::HighwayRoute);
        if street_like && !has(&self.tags, Tag::StreetNumber) && !self.zero_number {
            self.warn(Warning::MissingStreetNumber);
        }
        // "123 UNIT 1": a house number and a unit, no street.
        let unit = self
            .tags
            .iter()
            .any(|t| matches!(t, Tag::UnitType(_) | Tag::UnitNumber(_)));
        if unit && !street_like && has(&self.tags, Tag::StreetNumber) {
            self.warn(Warning::MissingStreetName);
        }
        if has(&self.tags, Tag::Unparsed) {
            self.warn(Warning::UnparsedRemainder);
        }
        if self.hint_used.get() {
            self.warn(Warning::DefaultStateApplied);
            self.decide(Decision::DefaultState);
        }
        if let Some(k) = self.incomplete {
            let tok = &self.toks[k];
            let seg_len = self.lens.seg.get(tok.seg).copied().unwrap_or(0);
            let at_end = k + 1 == self.toks.len();
            let whole = self.lens.whole.unwrap_or(0);
            if seg_len >= TRUNCATION_MIN_LEN || (at_end && whole >= TRUNCATION_MIN_LEN) {
                self.warn(Warning::PossiblyTruncated);
            }
        }
        Tagged {
            tags: self.tags,
            warnings: self.warnings,
            decisions: self.decisions,
        }
    }
}

/// Tag a single-line input.
pub(crate) fn tag_line(
    toks: &[Token],
    lex: Lexicons,
    lens: LineLengths,
    hint: Option<&'static str>,
) -> Tagged {
    let mut t = Tagger::new(toks, lex, lens, hint);
    let (street_end, city) = t.last_line();
    let lo = t.drop_truncated_copy(0, street_end);
    let street_end = t.drop_repeated_zip(lo, street_end);
    let street_end = t.drop_city_copy(lo, street_end, city);
    t.delivery(lo, street_end);
    t.finish()
}

/// Tag [`crate::Fields`] input. `ranges` gives each field's token range;
/// fields were tokenized in order, so the ranges are contiguous.
pub(crate) fn tag_fields(
    toks: &[Token],
    ranges: &[(FieldName, Range<usize>)],
    lex: Lexicons,
    lens: LineLengths,
    hint: Option<&'static str>,
) -> Tagged {
    let get = |f: FieldName| {
        ranges
            .iter()
            .find(|(n, _)| *n == f)
            .map_or(0..0, |(_, r)| r.clone())
    };
    let (l1, l2) = (get(FieldName::Line1), get(FieldName::Line2));
    let (city, state, zip) = (
        get(FieldName::City),
        get(FieldName::State),
        get(FieldName::Zip),
    );
    let mut line = l1.start.min(l2.start)..l1.end.max(l2.end);
    let mut t = Tagger::new(toks, lex, lens, hint);
    // Line 2 that repeats the end of line 1 (`9453 LUTOO LANE` / `LUTOO
    // LANE`, seen in Florida address data) is a copy.
    if !l2.is_empty() && l2.start == l1.end && l2.len() <= l1.len() {
        let same = toks[l2.clone()]
            .iter()
            .zip(&toks[l1.end - l2.len()..l1.end])
            .all(|(a, b)| a.text == b.text);
        if same {
            t.set(l2.clone(), Tag::Skip);
            line = l1.clone();
        }
    }
    line.start = t.drop_truncated_copy(line.start, line.end);
    if city.is_empty() && state.is_empty() && zip.is_empty() {
        // Everything is on the address lines: read them like one line.
        let (street_end, c) = t.last_line_in(line.start, line.end);
        let street_end = t.drop_repeated_zip(line.start, street_end);
        let street_end = t.drop_city_copy(line.start, street_end, c);
        t.delivery(line.start, street_end);
        return t.finish();
    }
    let city_range = t.city_field(city, state.is_empty());
    t.state_field(state);
    t.zip_field(zip.clone());
    let mut end = line.end;
    if zip.is_empty() {
        end = t.trailing_zip(line.start, end);
    } else {
        end = t.drop_repeated_zip(line.start, end);
    }
    let end = t.drop_repeated_state(line.start, end);
    let end = t.drop_city_copy(line.start, end, city_range);
    t.delivery(line.start, end);
    t.finish()
}

#[cfg(test)]
mod tests {
    use super::Decision::*;
    use super::*;
    use crate::clean::clean;
    use crate::tokenize::tokenize;

    fn decisions(input: &str, hint: Option<&'static str>) -> Vec<Decision> {
        let lex = Lexicons::builtin();
        let toks = tokenize(&clean(input), None, 0, &lex);
        let lens = LineLengths {
            seg: Vec::new(),
            whole: Some(input.len()),
        };
        tag_line(&toks, lex, lens, hint).decisions
    }

    #[test]
    fn a_plain_parse_records_only_how_the_city_was_found() {
        for input in [
            "123 MAIN ST, MIAMI, FL 33101",
            "123 MAIN ST APT 4, MIAMI, FL 33101",
            "PO BOX 5, MIAMI, FL 33101",
            "123 MAIN ST, MIAMI",
        ] {
            assert_eq!(decisions(input, None), vec![CityByComma], "{input}");
        }
        assert_eq!(decisions("123 MAIN ST, FL 33101", None), vec![]);
    }

    #[test]
    fn a_city_on_one_line_by_suffix_or_place() {
        assert_eq!(
            decisions("12 LAKE SHORE DR ZYQXV FL 33101", None),
            vec![CityBySuffix]
        );
        #[cfg(feature = "places-us")]
        assert_eq!(
            decisions("123 MAIN ST WEST PALM BEACH FL 33401", None),
            vec![CityByPlace]
        );
    }

    #[test]
    fn a_state_with_no_zip() {
        assert_eq!(
            decisions("123 MAIN ST, ZYQXV FL", None),
            vec![StateWithoutZip, CityByComma]
        );
    }

    #[test]
    fn bare_units_and_the_number_letter() {
        assert_eq!(
            decisions("100 OAK DR 706, MIAMI, FL 33101", None),
            vec![CityByComma, BareUnit]
        );
        assert_eq!(
            decisions("100 OAK DR V 21, MIAMI, FL 33101", None),
            vec![CityByComma, BareUnit]
        );
        assert_eq!(
            decisions("2309 1ST AVE NO, 403, MIAMI, FL 33101", None),
            vec![CityByComma, BareUnit]
        );
        assert_eq!(
            decisions("428 B SOUTH MAIN ST, MIAMI, FL 33101", None),
            vec![CityByComma, NumberLetter]
        );
        // A designator makes the unit certain.
        assert_eq!(
            decisions("100 OAK DR APT 706, MIAMI, FL 33101", None),
            vec![CityByComma]
        );
    }

    #[test]
    #[cfg(all(feature = "places-us", feature = "state-fl"))]
    fn the_default_state_hint() {
        assert_eq!(
            decisions("281 EVELYN DR DOTHAN", Some("AL")),
            vec![CityByPlace, DefaultState]
        );
        assert_eq!(decisions("281 EVELYN DR DOTHAN", None), vec![]);
        assert_eq!(decisions("22200 SR 60", Some("FL")), vec![DefaultState]);
        assert_eq!(decisions("22200 SR 60", Some("OH")), vec![]);
        // The input's own state wins; the hint decides nothing.
        assert_eq!(
            decisions("500 OAK ST, ATLANTA, GA 30301", Some("FL")),
            vec![CityByComma]
        );
    }
}
