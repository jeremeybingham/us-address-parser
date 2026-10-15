//! Rule-based tagger (pipeline step 4, item 13).
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

use crate::lexicon::Lexicons;
use crate::tokenize::Token;
use crate::types::{FieldName, Warning};

/// Lines at least this long that end in something incomplete (a cut-off
/// ZIP, a designator with no number, a cut-off word) get
/// [`Warning::PossiblyTruncated`]. County sites cut address lines at about
/// 30 characters (`docs/prior-art.md`).
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

pub(crate) struct Tagged {
    pub tags: Vec<Tag>,
    pub warnings: Vec<Warning>,
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
    lex: Lexicons,
    /// A token where the input looks cut off, if any.
    incomplete: Option<usize>,
    zero_number: bool,
    lens: LineLengths,
}

impl<'a> Tagger<'a> {
    fn new(toks: &'a [Token], lex: Lexicons, lens: LineLengths) -> Self {
        Self {
            toks,
            tags: vec![Tag::Untagged; toks.len()],
            warnings: Vec::new(),
            lex,
            incomplete: None,
            zero_number: false,
            lens,
        }
    }

    fn warn(&mut self, w: Warning) {
        if !self.warnings.contains(&w) {
            self.warnings.push(w);
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
        }
    }
}

/// Tag a single-line input.
pub(crate) fn tag_line(toks: &[Token], lex: Lexicons, lens: LineLengths) -> Tagged {
    let mut t = Tagger::new(toks, lex, lens);
    let (street_end, city) = t.last_line();
    let street_end = t.drop_repeated_zip(0, street_end);
    let street_end = t.drop_city_copy(0, street_end, city);
    t.delivery(0, street_end);
    t.finish()
}

/// Tag [`crate::Fields`] input. `ranges` gives each field's token range;
/// fields were tokenized in order, so the ranges are contiguous.
pub(crate) fn tag_fields(
    toks: &[Token],
    ranges: &[(FieldName, Range<usize>)],
    lex: Lexicons,
    lens: LineLengths,
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
    let mut t = Tagger::new(toks, lex, lens);
    // Line 2 that repeats the end of line 1 (`9453 LUTOO LANE` / `LUTOO
    // LANE`, DBPR data) is a copy.
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
