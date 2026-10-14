//! Tagging rules. Sources for each rule are in `docs/usps-pub28-notes.md`
//! (Pub 28) and `docs/prior-art.md` (shapes seen in county data).

use std::ops::Range;

use super::{Tag, Tagger};
use crate::classify::{
    all_digits, has_digit, house_number, is_cut_designator, is_fraction, is_glued_ordinal,
    is_letter, is_route_value, is_slash_pair, is_unit_value, join, longest_at, longest_ending,
    zip_parts, NumberShape,
};
use crate::lexicon::{MatchKind, Special};
use crate::types::{StateKind, Warning};

/// A special delivery line: the tags to apply and where it ends.
struct SpecialMatch {
    tags: Vec<(Range<usize>, Tag)>,
    end: usize,
    /// Whether a unit may follow (PO Box, rural route).
    units: bool,
}

/// A highway street name found in the delivery line.
struct HighwayAt {
    pre: Range<usize>,
    prefix: Range<usize>,
    route: usize,
}

impl Tagger<'_> {
    // ---- last line: ZIP, state, city ---------------------------------------

    /// Tag the last line of a single-line input. Returns the end of the
    /// delivery-line tokens and the city's tokens, if any.
    pub(super) fn last_line(&mut self) -> (usize, Option<Range<usize>>) {
        self.last_line_in(0, self.toks.len())
    }

    /// [`Self::last_line`] over tokens `lo..hi`.
    pub(super) fn last_line_in(&mut self, lo: usize, hi: usize) -> (usize, Option<Range<usize>>) {
        if lo >= hi {
            return (hi, None);
        }
        let toks = self.toks;
        let mut end = hi;
        let last = &toks[hi - 1];
        let after_hash = hi - 1 > lo && toks[hi - 2].is_hash();
        let mut zip = false;
        let mut bad_zip = false;

        // ZIP: the last token. A number after '#' is a unit (`#11176`).
        if zip_parts(&last.text).is_some() && !after_hash && !last.paren {
            self.set(hi - 1..hi, Tag::Zip);
            end -= 1;
            zip = true;
        } else if all_digits(&last.text) && !after_hash && hi - 1 > lo {
            // A cut-off ZIP right after a state: `FL 3`, `FL 320`.
            let seg_start = self.seg_start(hi - 1).max(lo);
            if let Some((_, w)) =
                longest_ending(toks, seg_start, hi - 1, |s| self.state_ok(s, true))
            {
                let starts_segment = hi - 1 - w == seg_start && seg_start > lo;
                let long = self.lens.whole.unwrap_or(0) >= super::TRUNCATION_MIN_LEN;
                if starts_segment || long {
                    self.set(hi - 1..hi, Tag::Unparsed);
                    self.warn(Warning::InvalidZip);
                    self.incomplete = Some(hi - 1);
                    end -= 1;
                    bad_zip = true;
                }
            }
        }

        // State: right before the ZIP, or first in the ZIP's segment
        // (anything between is unparsed). With no ZIP, only a state that
        // ends a later segment ("..., MIAMI FL").
        let mut ll_start = end;
        let mut state: Option<Range<usize>> = None;
        if end > lo {
            let seg_start = self.seg_start(end - 1).max(lo);
            if zip || bad_zip {
                if let Some((_, w)) =
                    longest_ending(toks, seg_start, end, |s| self.state_ok(s, true))
                {
                    state = Some(end - w..end);
                } else if let Some((_, w)) =
                    longest_at(toks, seg_start, end, |s| self.state_ok(s, false))
                {
                    self.set(seg_start + w..end, Tag::Unparsed);
                    state = Some(seg_start..seg_start + w);
                }
            } else if let Some((_, w)) =
                longest_ending(toks, seg_start, end, |s| self.state_ok(s, false))
            {
                // No ZIP: a state ending a later segment ("..., MIAMI FL"),
                // or ending a single line after a city ("... ST MIAMI FL",
                // not "... ST FL", where FL may be a floor).
                let later_seg = seg_start > lo;
                if later_seg || self.city_after_street(lo, end - w).is_some() {
                    state = Some(end - w..end);
                }
            }
        }
        if let Some(r) = &state {
            self.set(r.clone(), Tag::State);
            ll_start = r.start;
        }
        if !zip && !bad_zip && state.is_none() {
            // "123 MAIN ST, MIAMI": a last segment of plain words after a
            // street is the city.
            let cs = self.seg_start(hi - 1).max(lo);
            if cs > lo && self.looks_like_city(cs, hi) && !self.starts_unit(cs, hi) {
                self.set(cs..hi, Tag::City);
                return (cs, Some(cs..hi));
            }
            return (hi, None);
        }

        // APO / FPO / DPO with a military state code takes the city's place.
        let military_state = state.as_ref().is_some_and(|r| {
            self.lex
                .state(&join(&toks[r.clone()]))
                .is_some_and(|l| l.value.kind() == StateKind::Military)
        });
        if military_state && ll_start > lo {
            let k = ll_start - 1;
            if self.lex.military_post_office(self.text(k)).is_some() {
                self.set(k..k + 1, Tag::MilitaryPostOffice);
                return (k, None);
            }
        }
        if ll_start == lo {
            return (lo, None);
        }

        // City.
        let prev = ll_start - 1;
        let city = if toks[prev].seg != toks[ll_start].seg {
            // The last line starts its segment: the city is the segment before.
            let cs = self.seg_start(prev).max(lo);
            let first_seg = toks[cs].seg == toks[lo].seg;
            (!first_seg || self.looks_like_city(cs, ll_start)).then_some(cs..ll_start)
        } else {
            let cs = self.seg_start(prev).max(lo);
            if toks[cs].seg != toks[lo].seg {
                Some(cs..ll_start)
            } else {
                // One line with no commas: the city follows the street.
                self.city_after_street(lo, ll_start).map(|c| c..ll_start)
            }
        };
        match city {
            Some(c) if !c.is_empty() => {
                self.set(c.clone(), Tag::City);
                (c.start, Some(c))
            }
            _ => (ll_start, None),
        }
    }

    /// A state spelling. `loose` also accepts traditional abbreviations
    /// (FLA), which are only safe right before a ZIP.
    fn state_ok(&self, s: &str, loose: bool) -> Option<()> {
        let l = self.lex.state(s)?;
        (loose || l.kind != MatchKind::Variant).then_some(())
    }

    /// Whether the first segment, when it is all that comes before the last
    /// line, is a city rather than a street: no digits, and not a PO Box,
    /// route, general delivery or military line.
    fn looks_like_city(&self, lo: usize, hi: usize) -> bool {
        let toks = &self.toks[lo..hi];
        !toks.iter().any(|t| has_digit(&t.text))
            && longest_at(self.toks, lo, hi, |s| self.lex.special(s)).is_none()
            && longest_at(self.toks, lo, hi, |s| self.lex.military(s)).is_none()
    }

    /// Whether `lo..hi` starts with a unit designator or `#` (a second
    /// address line such as `STE 200` or `REAR`).
    fn starts_unit(&self, lo: usize, hi: usize) -> bool {
        self.toks[lo].is_hash() || longest_at(self.toks, lo, hi, |s| self.lex.unit(s)).is_some()
    }

    /// In a line with no commas, where the city starts: after the last
    /// street suffix, its postdirectional and a unit. With no suffix, the
    /// whole run is the city only if it has no digits (`TAMPA 33616`).
    fn city_after_street(&self, lo: usize, hi: usize) -> Option<usize> {
        if lo >= hi {
            return None;
        }
        // "GENERAL DELIVERY JUNEAU", "PO BOX 5 MIAMI".
        if let Some(m) = self.match_special(lo, hi) {
            return (m.end < hi).then_some(m.end);
        }
        // The street ends at a suffix with a name before it. Prefer the
        // first one followed by a word that is not also a suffix
        // ("12 LAKE SHORE DR CHICAGO"), else the last one.
        let is_suffix = |k: usize| self.lex.suffix(self.text(k)).is_some();
        let first_number = house_number(self.text(lo)).is_some();
        let min = if first_number { lo + 2 } else { lo + 1 };
        let suffix = (min..hi)
            .find(|&k| is_suffix(k) && k + 1 < hi && !is_suffix(k + 1))
            .or_else(|| (min..hi).rev().find(|&k| is_suffix(k)));
        let Some(p) = suffix else {
            let digits = self.toks[lo..hi].iter().any(|t| has_digit(&t.text));
            return (!digits).then_some(lo);
        };
        let mut c = p + 1;
        if c + 1 < hi && self.lex.directional(self.text(c)).is_some() {
            c += 1;
        }
        if c + 1 < hi
            && (self.toks[c].is_hash() || self.lex.unit(self.text(c)).is_some())
            && is_unit_value(self.text(c + 1))
        {
            c += 2;
        }
        (c < hi).then_some(c)
    }

    /// Drop a copy of the city at the end of the delivery line, possibly cut
    /// off: `85231 BOSTWICK DR FERNANDINA B` before `FERNANDINA BEACH`,
    /// `SEVENTH AV HILLIARD` before `HILLIARD` (`docs/prior-art.md`).
    /// Returns the new end of the delivery line.
    pub(super) fn drop_city_copy(
        &mut self,
        lo: usize,
        hi: usize,
        city: Option<Range<usize>>,
    ) -> usize {
        let Some(city) = city else { return hi };
        let words: Vec<&str> = self.toks[city].iter().map(|t| t.text.as_str()).collect();
        for k in (1..=words.len()).rev() {
            if hi < lo + k + 1 {
                continue;
            }
            let tail = &self.toks[hi - k..hi];
            // "100 MAIN ST" before "ST PETERSBURG": a suffix is the street's.
            if self.lex.suffix(&tail[0].text).is_some() {
                continue;
            }
            let matches = tail.iter().enumerate().all(|(n, t)| {
                if n + 1 == k {
                    words[n].starts_with(t.text.as_str())
                } else {
                    t.text == words[n]
                }
            });
            if matches {
                self.set(hi - k..hi, Tag::Skip);
                return hi - k;
            }
        }
        hi
    }

    // ---- multi-field input -------------------------------------------------

    /// Tag the city field. A trailing state (`BOCA RATON, FL`) is taken as
    /// the state when the state field is empty. Returns the city's tokens.
    pub(super) fn city_field(&mut self, r: Range<usize>, take_state: bool) -> Option<Range<usize>> {
        if r.is_empty() {
            return None;
        }
        if r.len() == 1 && self.lex.military_post_office(self.text(r.start)).is_some() {
            self.set(r, Tag::MilitaryPostOffice);
            return None;
        }
        let mut end = r.end;
        if take_state {
            if let Some((_, w)) =
                longest_ending(self.toks, r.start + 1, r.end, |s| self.state_ok(s, false))
            {
                self.set(r.end - w..r.end, Tag::State);
                end -= w;
            }
        }
        self.set(r.start..end, Tag::City);
        Some(r.start..end)
    }

    pub(super) fn state_field(&mut self, r: Range<usize>) {
        if r.is_empty() {
            return;
        }
        let whole = longest_at(self.toks, r.start, r.end, |s| self.state_ok(s, true))
            .filter(|(_, w)| *w == r.len());
        if whole.is_some() {
            self.set(r, Tag::State);
        } else {
            self.set(r, Tag::Unparsed);
            self.warn(Warning::UnknownState);
        }
    }

    pub(super) fn zip_field(&mut self, r: Range<usize>) {
        if r.is_empty() {
            return;
        }
        if r.len() == 1 && zip_parts(self.text(r.start)).is_some() {
            self.set(r, Tag::Zip);
        } else {
            self.set(r, Tag::Unparsed);
            self.warn(Warning::InvalidZip);
        }
    }

    /// A ZIP at the end of the address line when the ZIP field is empty
    /// (`7702 LILLIAN HWY 32506`). Returns the new end of the line.
    pub(super) fn trailing_zip(&mut self, lo: usize, hi: usize) -> usize {
        if hi >= lo + 2 && zip_parts(self.text(hi - 1)).is_some() && !self.toks[hi - 2].is_hash() {
            self.set(hi - 1..hi, Tag::Zip);
            return hi - 1;
        }
        hi
    }

    // ---- delivery line -----------------------------------------------------

    pub(super) fn delivery(&mut self, lo: usize, hi: usize) {
        if lo >= hi || self.placeholder(lo, hi) {
            return;
        }
        let mut lo = lo;
        while lo < hi && self.toks[lo].is_dash() {
            self.set(lo..lo + 1, Tag::Skip);
            lo += 1;
        }
        // "N/A - MAGNOLIA AVE": a placeholder prefix before a dash.
        if let Some(d) = (lo..hi).find(|&k| self.toks[k].is_dash()) {
            if d > lo && self.lex.is_placeholder(&join(&self.toks[lo..d])) {
                let rest = &self.toks[d + 1..hi];
                // "NO ADDRESS - 061S311301002007": a trailing parcel code.
                if rest.len() == 1 && has_digit(&rest[0].text) {
                    self.set(lo..hi, Tag::Skip);
                    self.warn(Warning::Placeholder);
                    return;
                }
                self.set(lo..d + 1, Tag::Skip);
                lo = d + 1;
            }
        }
        if lo >= hi || self.special_line(lo, hi) {
            return;
        }
        self.street(lo, hi);
    }

    fn placeholder(&mut self, lo: usize, hi: usize) -> bool {
        if self.lex.is_placeholder(&join(&self.toks[lo..hi])) {
            self.set(lo..hi, Tag::Skip);
            self.warn(Warning::Placeholder);
            return true;
        }
        false
    }

    /// A `#`-optional value at `k`: returns the value's index.
    fn value_at(&self, k: usize, hi: usize) -> Option<(usize, bool)> {
        let hash = k < hi && self.toks[k].is_hash();
        let v = if hash { k + 1 } else { k };
        (v < hi && is_unit_value(self.text(v))).then_some((v, hash))
    }

    /// General delivery, PO Box, rural route, highway contract route and
    /// military box lines. Returns whether the line was one of these.
    fn special_line(&mut self, lo: usize, hi: usize) -> bool {
        let Some(m) = self.match_special(lo, hi) else {
            return false;
        };
        for (r, tag) in m.tags {
            self.set(r, tag);
        }
        if m.units {
            self.units(m.end, m.end, hi, false);
        }
        true
    }

    /// Match a special delivery line starting at `lo`, without tagging.
    fn match_special(&self, lo: usize, hi: usize) -> Option<SpecialMatch> {
        let lex = self.lex;
        let special = longest_at(self.toks, lo, hi, |s| lex.special(s));
        let mut tags = Vec::new();
        let value = |tags: &mut Vec<(Range<usize>, Tag)>, v: usize, hash: bool, tag: Tag| {
            if hash {
                tags.push((v - 1..v, Tag::Skip));
            }
            tags.push((v..v + 1, tag));
        };

        if let Some((l, w)) = special {
            match l.value {
                // Pub 28 section 261.
                Special::GeneralDelivery => {
                    tags.push((lo..lo + w, Tag::GeneralDelivery));
                    return Some(SpecialMatch {
                        tags,
                        end: lo + w,
                        units: false,
                    });
                }
                // Pub 28 sections 281 and 283.
                Special::PoBox => {
                    if let Some((v, hash)) = self.value_at(lo + w, hi) {
                        tags.push((lo..lo + w, Tag::PoBox));
                        value(&mut tags, v, hash, Tag::PoBoxNumber);
                        return Some(SpecialMatch {
                            tags,
                            end: v + 1,
                            units: true,
                        });
                    }
                }
                // Pub 28 sections 241-244 and 251-253. Forms that are also
                // street words (RD, RTE, ROUTE) count only with a BOX after
                // the route.
                Special::RuralRoute | Special::HighwayContract => {
                    let form = join(&self.toks[lo..lo + w]);
                    let ambiguous = lex.highway(&form).is_some() || lex.suffix(&form).is_some();
                    if let Some((route, hash)) = self.value_at(lo + w, hi) {
                        let bx = self.box_part(route + 1, hi);
                        if bx.is_some() || !ambiguous {
                            tags.push((lo..lo + w, Tag::Route));
                            value(&mut tags, route, hash, Tag::RouteNumber);
                            let mut end = route + 1;
                            if let Some((word, v)) = bx {
                                tags.push((word.start..v, Tag::Skip));
                                tags.push((v..v + 1, Tag::BoxNumber));
                                end = v + 1;
                            }
                            return Some(SpecialMatch {
                                tags,
                                end,
                                units: true,
                            });
                        }
                    }
                }
                Special::Box => {}
            }
        }

        // Pub 28 sections 238.1 and 239: TYPE ## BOX ##.
        let (_, w) = longest_at(self.toks, lo, hi, |s| lex.military(s))?;
        let (num, hash) = self.value_at(lo + w, hi)?;
        let (word, v) = self.box_part(num + 1, hi)?;
        tags.push((lo..lo + w, Tag::MilitaryKind));
        value(&mut tags, num, hash, Tag::MilitaryNumber);
        tags.push((word.start..v, Tag::Skip));
        tags.push((v..v + 1, Tag::MilitaryBoxNumber));
        Some(SpecialMatch {
            tags,
            end: v + 1,
            units: false,
        })
    }

    /// `BOX 152` (or `PO BOX`, `#`) at `k`: the box word's range and the
    /// value's index.
    fn box_part(&self, k: usize, hi: usize) -> Option<(Range<usize>, usize)> {
        let lex = self.lex;
        let (_, w) = longest_at(self.toks, k, hi, |s| {
            lex.special(s)
                .filter(|l| matches!(l.value, Special::Box | Special::PoBox))
        })?;
        let (v, _) = self.value_at(k + w, hi)?;
        Some((k..k + w, v))
    }

    /// A street address: number, highway or name, suffix, directionals and
    /// up to two units.
    fn street(&mut self, lo: usize, hi: usize) {
        let mut i = lo;
        let t0 = self.text(i);
        if is_glued_ordinal(t0) {
            // "409742ND SQ": 4097 42ND or 40974 2ND. Don't guess.
            self.warn(Warning::AmbiguousStreetNumber);
            self.set(lo..hi, Tag::Unparsed);
            return;
        }
        if is_slash_pair(t0) {
            // "5210/5212 BRISTO ST"
            self.warn(Warning::MultipleAddresses);
            self.set(i..i + 1, Tag::Unparsed);
            i += 1;
        } else if let Some(shape) = house_number(t0).filter(|_| hi > lo + 1) {
            if t0.bytes().all(|c| c == b'0') {
                self.set(i..i + 1, Tag::Skip);
                self.zero_number = true;
                self.warn(Warning::ZeroStreetNumber);
            } else {
                self.set(i..i + 1, Tag::StreetNumber);
                if shape == NumberShape::Range {
                    self.warn(Warning::MultipleAddresses);
                }
            }
            i += 1;
            if i < hi && is_fraction(self.text(i)) {
                self.set(i..i + 1, Tag::StreetNumber);
                i += 1;
            }
        }
        if i >= hi {
            return;
        }

        if let Some(h) = self.highway_at(i, hi) {
            let floor = h.route + 1;
            let j = self.units(floor, floor, hi, true);
            self.set(h.pre.clone(), Tag::PreDir);
            self.set(h.prefix.clone(), Tag::HighwayPrefix);
            self.set(h.route..h.route + 1, Tag::HighwayRoute);
            self.street_tail(floor, j);
            return;
        }

        let j = self.units(i, i, hi, true);
        // The name and suffix come from the first line; whatever is left of
        // a later line ("..., BLDG 5 ROOM") is not part of the name.
        let line_end = (i..j)
            .find(|&k| self.toks[k].seg != self.toks[i].seg)
            .unwrap_or(j);
        self.street_body(i, line_end, hi.min(line_end));
    }

    /// Pub 28 section 237 and Appendix F: a highway prefix and route right
    /// after the house number (and an optional predirectional).
    fn highway_at(&self, i: usize, hi: usize) -> Option<HighwayAt> {
        let lex = self.lex;
        let try_at = |p: usize| {
            let (_, w) = longest_at(self.toks, p, hi, |s| lex.highway(s))?;
            let route = p + w;
            (route < hi && is_route_value(self.text(route))).then_some((p..p + w, route))
        };
        if let Some((prefix, route)) = try_at(i) {
            return Some(HighwayAt {
                pre: i..i,
                prefix,
                route,
            });
        }
        let (_, w) = longest_at(self.toks, i, hi, |s| lex.directional(s))?;
        let (prefix, route) = try_at(i + w)?;
        Some(HighwayAt {
            pre: i..i + w,
            prefix,
            route,
        })
    }

    /// After a highway route: a postdirectional and a suffix, anything else
    /// is part of the name (`HWY 66 FRONTAGE RD`).
    fn street_tail(&mut self, i: usize, j: usize) {
        let mut j = j;
        if let Some((_, w)) = longest_ending(self.toks, i, j, |s| self.lex.directional(s)) {
            self.set(j - w..j, Tag::PostDir);
            j -= w;
        }
        if j > i && self.lex.suffix(self.text(j - 1)).is_some() {
            self.set(j - 1..j, Tag::Suffix);
            j -= 1;
        }
        self.set(i..j, Tag::Name);
    }

    /// Predirectional, name, suffix and postdirectional in `i..j` (Pub 28
    /// section 233). `hi` is the end of the whole delivery line.
    fn street_body(&mut self, i: usize, j: usize, hi: usize) {
        let lex = self.lex;
        let (mut i, mut j) = (i, j);
        if i >= j {
            return;
        }
        // Postdirectional, unless it is a letter naming the street right
        // after a lone suffix word ("AVENUE N", like "AVENUE G").
        if j >= i + 2 {
            if let Some((_, w)) = longest_ending(self.toks, i + 1, j, |s| lex.directional(s)) {
                let k = j - w;
                let letter = self.text(k).len() == 1;
                let lone_suffix = k == i + 1 && lex.suffix(self.text(i)).is_some();
                if !(letter && lone_suffix) {
                    self.set(k..j, Tag::PostDir);
                    j = k;
                }
            }
        }
        // Suffix, when a name is left before it.
        let mut suffix_taken = false;
        if j >= i + 2 && lex.suffix(self.text(j - 1)).is_some() {
            self.set(j - 1..j, Tag::Suffix);
            j -= 1;
            suffix_taken = true;
        }
        // Predirectional, when a name is left after it.
        if let Some((_, w)) = longest_at(self.toks, i, j, |s| lex.directional(s)) {
            if i + w < j {
                self.set(i..i + w, Tag::PreDir);
                i += w;
            }
        }
        self.set(i..j, Tag::Name);
        // A name with no suffix at the very end may be cut off ("CUTOF").
        if !suffix_taken && j == hi && j > i && self.tags[j - 1] == Tag::Name {
            self.incomplete = Some(j - 1);
        }
    }

    /// Whether a street suffix (with a name before it) ends right before
    /// token `k`, allowing a postdirectional in between. Highway words are
    /// excluded: after HWY a number is the route, not a unit.
    fn suffix_before(&self, name_lo: usize, k: usize) -> bool {
        let lex = self.lex;
        let mut s = k;
        if s > name_lo && lex.directional(self.text(s - 1)).is_some() {
            s -= 1;
        }
        if s == 0 || s <= name_lo {
            return false;
        }
        let sfx = s - 1;
        let word = self.text(sfx);
        let highway = lex
            .highway(word)
            .is_some_and(|h| h.value.standard() != "ROAD");
        // A leading directional is the predirectional, not the name:
        // "147 SE AVENUE A" has no name before AVENUE, so A is not a unit.
        let mut name_start = name_lo;
        if lex.directional(self.text(name_lo)).is_some() {
            name_start += 1;
        }
        lex.suffix(word).is_some() && !highway && sfx > name_start
    }

    /// Up to two units at the end of `floor..hi`, read from the right.
    /// `name_lo` is where the street name starts (for the bare-unit rule).
    /// Returns the new end of the street.
    fn units(&mut self, floor: usize, name_lo: usize, hi: usize, bare: bool) -> usize {
        let lex = self.lex;
        let mut j = hi;
        let mut found: Vec<(Range<usize>, Range<usize>)> = Vec::new();
        while found.len() < 2 && j > floor {
            let last = j - 1;
            let t = self.text(last);
            // "(110)" is a unit; "(OFF)" is not.
            if self.toks[last].paren {
                if is_unit_value(t) {
                    found.push((last..last, last..j));
                } else {
                    self.set(last..j, Tag::Unparsed);
                }
                j = last;
                continue;
            }
            // A designator cut off at the line limit: "... DR UNI".
            if found.is_empty()
                && j == hi
                && is_cut_designator(t, &lex)
                && self.suffix_before(name_lo, last)
            {
                self.set(last..j, Tag::Unparsed);
                self.incomplete = Some(last);
                j = last;
                continue;
            }
            // "#" with nothing after it.
            if self.toks[last].is_hash() {
                self.set(last..j, Tag::Skip);
                self.warn(Warning::MissingUnitNumber);
                self.incomplete = Some(last);
                found.push((last..last, last..last));
                j = last;
                continue;
            }
            // "# 309", "UNIT 203", "LOT H28".
            if is_unit_value(t) && last > floor {
                if self.toks[last - 1].is_hash() {
                    self.set(last - 1..last, Tag::Skip);
                    // "UNIT # 4": the designator before the '#' is this unit's.
                    let hash = last - 1;
                    match longest_ending(self.toks, floor, hash, |s| lex.unit(s)) {
                        Some((_, w)) => {
                            found.push((hash - w..hash, last..j));
                            j = hash - w;
                        }
                        None => {
                            found.push((hash..hash, last..j));
                            j = hash;
                        }
                    }
                    continue;
                }
                if let Some((_, w)) = longest_ending(self.toks, floor, last, |s| lex.unit(s)) {
                    found.push((last - w..last, last..j));
                    j = last - w;
                    continue;
                }
            }
            // A designator with nothing after it: "... TRL UNIT".
            if let Some((u, w)) = longest_ending(self.toks, floor + 1, j, |s| lex.unit(s)) {
                if j - w > name_lo + 1 {
                    if u.value.range_required() {
                        self.warn(Warning::MissingUnitNumber);
                        self.incomplete = Some(last);
                    }
                    found.push((j - w..j, j..j));
                    j -= w;
                    continue;
                }
            }
            // A bare unit after the suffix: "DR 706", "DR B", "DR V 21".
            if bare && found.is_empty() {
                if last > floor
                    && all_digits(t)
                    && is_letter(self.text(last - 1))
                    && self.suffix_before(name_lo, last - 1)
                {
                    found.push((last - 1..last - 1, last - 1..j));
                    j = last - 1;
                    continue;
                }
                // A directional letter here is the postdirectional ("PARK AVE S").
                let dir = lex.directional(t).is_some();
                if is_unit_value(t) && !dir && self.suffix_before(name_lo, last) {
                    found.push((last..last, last..j));
                    j = last;
                    continue;
                }
            }
            break;
        }
        // Found right to left; the leftmost is the first unit.
        let n = found.len();
        for (k, (ty, num)) in found.into_iter().enumerate() {
            let which = u8::from(n == 2 && k == 0);
            self.set(ty, Tag::UnitType(which));
            self.set(num, Tag::UnitNumber(which));
        }
        j
    }
}
