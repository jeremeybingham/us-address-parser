//! Tagging rules. Sources for each rule are in `docs/usps-pub28-notes.md`
//! (Pub 28) and `docs/prior-art.md` (shapes seen in county data).

use std::ops::Range;

use super::{PlaceCtx, Tag, Tagger};
use crate::classify::{
    all_digits, has_digit, house_number, is_cut_designator, is_fraction, is_glued_ordinal,
    is_grid_number, is_letter, is_ordinal, is_route_value, is_slash_pair, is_unit_value, join,
    longest_at, longest_ending, zip_parts, NumberShape,
};
use crate::lexicon::{MatchKind, Special, MAX_PLACE_WORDS};
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
    /// A state before the prefix (`KY STATE HIGHWAY 1`); empty if none.
    state: Range<usize>,
    /// Empty when a state stands for the prefix (`KENTUCKY 440`).
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
                if later_seg
                    || self.city_after_street(lo, end - w).is_some()
                    || self.state_alone(lo, end - w, end)
                {
                    state = Some(end - w..end);
                }
            }
        }
        // "12 SIMON CT 32505": on one line with no commas, a state that is
        // also a street suffix, with no city before it, is the suffix.
        if let Some(r) = state.clone() {
            let text = join(&toks[r.clone()]);
            let one_line = toks[r.start].seg == toks[lo].seg;
            // "8517 CEDAR BARK CIR KY": after an abbreviated suffix it is
            // the state.
            let after_suffix = r.start > lo && self.abbreviated_suffix(r.start - 1);
            if one_line
                && self.lex.suffix(&text).is_some()
                && !after_suffix
                && self.city_after_street(lo, r.start).is_none()
            {
                state = None;
            }
        }
        if let Some(r) = &state {
            self.set(r.clone(), Tag::State);
            ll_start = r.start;
        }
        self.places = self.place_ctx(state.clone(), zip.then_some(hi - 1));
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
            if !first_seg
                || (self.looks_like_city(cs, ll_start) && !self.known_street(cs, ll_start))
            {
                Some(cs..ll_start)
            } else {
                // "626 E KILBOURN AVE MILWAUKEE, WI 53202": the street and
                // the city share the first segment. A segment ending in a
                // suffix or directional is all street (`427 SOUTH LA SALLE
                // STREET, 60605`).
                let last = self.text(prev);
                let street_end =
                    self.lex.suffix(last).is_some() || self.lex.directional(last).is_some();
                if street_end {
                    None
                } else {
                    self.place_city(cs, ll_start)
                        .or_else(|| self.city_after_street(cs, ll_start))
                        .map(|c| c..ll_start)
                }
            }
        } else {
            let cs = self.seg_start(prev).max(lo);
            if toks[cs].seg != toks[lo].seg {
                // "..., # 234 SANTA FE NM": a unit starting the city's
                // segment belongs to the street.
                Some(self.after_unit(cs, ll_start)..ll_start)
            } else {
                // One line with no commas: the city follows the street. A
                // place the state pack knows wins over the suffix rule
                // (`MAIN ST WEST PALM BEACH`).
                self.place_city(lo, ll_start)
                    .or_else(|| self.city_after_street(lo, ll_start))
                    .map(|c| c..ll_start)
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

    /// The place context for the last line: the state tagged there, else the
    /// state of the ZIP's main place.
    fn place_ctx(&self, state: Option<Range<usize>>, zip: Option<usize>) -> Option<PlaceCtx> {
        let zip = zip
            .and_then(|k| zip_parts(self.text(k)))
            .and_then(|(z5, _)| self.lex.zip_places(z5));
        let state = match state {
            Some(r) => self.lex.state(&join(&self.toks[r]))?.value.code(),
            None => zip?.main().state(),
        };
        let zip = zip.filter(|z| z.main().state() == state);
        Some(PlaceCtx { state, zip })
    }

    /// `WINGATE ST, 32507`: with a state pack, a first segment of plain words
    /// that ends in a street suffix and is no place the pack knows is the
    /// street, not the city (`docs/prior-art.md`).
    fn known_street(&self, lo: usize, hi: usize) -> bool {
        let Some(ctx) = self.places else {
            return false;
        };
        hi > lo + 1
            && self.lex.suffix(self.text(hi - 1)).is_some()
            && self
                .lex
                .place(ctx.state, &join(&self.toks[lo..hi]))
                .is_none()
    }

    /// In a line with no commas, where the city starts by the state pack's
    /// place list: the longest run of words ending at `hi` that is a place,
    /// preferring one the ZIP belongs to. A listed misspelling counts only
    /// when the ZIP agrees. After a house number, a street name must be left
    /// before the city; with no number the whole run may be the city
    /// (`PALM BEACH 33480`).
    pub(super) fn place_city(&self, lo: usize, hi: usize) -> Option<usize> {
        let ctx = self.places?;
        let first_number = lo < hi && house_number(self.text(lo)).is_some();
        let min = if first_number { lo + 2 } else { lo };
        let mut best: Option<(usize, bool)> = None;
        for k in min.max(hi.saturating_sub(MAX_PLACE_WORDS))..hi {
            let run = &self.toks[k..hi];
            if run.iter().any(|t| has_digit(&t.text) || t.is_hash()) {
                continue;
            }
            let Some(hit) = self.lex.place(ctx.state, &join(run)) else {
                continue;
            };
            let agrees = ctx.zip.is_some_and(|z| z.contains(&hit.value));
            if hit.kind != MatchKind::Standard && !agrees {
                continue;
            }
            // Longest first, so a later run replaces it only if the ZIP
            // agrees with it and not with the longer one.
            if !matches!(best, Some((_, a)) if a || !agrees) {
                best = Some((k, agrees));
            }
        }
        best.map(|(k, _)| k)
    }

    /// A state spelling. `loose` also accepts traditional abbreviations
    /// (FLA), which are only safe right before a ZIP.
    fn state_ok(&self, s: &str, loose: bool) -> Option<()> {
        let l = self.lex.state(s)?;
        (loose || l.kind != MatchKind::Variant).then_some(())
    }

    /// "8517 CEDAR BARK CIR AL": with no ZIP and no city, a USPS state code
    /// or name (`s..e`) ending a one-line street. Never NE or FL (also a
    /// directional and a floor). A code that is also a suffix or highway word
    /// (CT, KY, LA, MT, PR, WY, FM) only right after a suffix written as its
    /// abbreviation (`CIR KY`, not `HIDDEN MEADOW CT`).
    fn state_alone(&self, lo: usize, s: usize, e: usize) -> bool {
        if s < lo + 2 {
            return false;
        }
        let text = join(&self.toks[s..e]);
        let Some(l) = self.lex.state(&text) else {
            return false;
        };
        if l.kind == MatchKind::Variant
            || l.value.kind() == StateKind::Military
            || self.lex.directional(&text).is_some()
            || self.lex.unit(&text).is_some()
        {
            return false;
        }
        let other_word = self.lex.suffix(&text).is_some() || self.lex.highway(&text).is_some();
        !other_word || self.abbreviated_suffix(s - 1)
    }

    /// Whether token `k` is a street suffix written as its Pub 28
    /// abbreviation, shorter than the word (`CIR`; not `CIRCLE`, `MEADOW`,
    /// or `PARK`, whose abbreviation is the word itself).
    fn abbreviated_suffix(&self, k: usize) -> bool {
        let t = self.text(k);
        self.lex
            .suffix(t)
            .is_some_and(|l| l.value.abbr() == t && l.value.name() != t)
    }

    /// The token after a unit (`# 234`, `STE 5`) starting at `k`, or `k` when
    /// there is no unit there or nothing would be left before `hi`.
    fn after_unit(&self, k: usize, hi: usize) -> usize {
        let end = if self.toks[k].is_hash() {
            self.value_at(k, hi).map(|(v, _)| v + 1)
        } else {
            longest_at(self.toks, k, hi, |s| self.lex.unit(s))
                .and_then(|(_, w)| self.value_at(k + w, hi))
                .map(|(v, _)| v + 1)
        };
        match end {
            Some(e) if e < hi => e,
            _ => k,
        }
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
        let is_suffix = |k: usize| self.lex.suffix(self.text(k)).is_some();
        let first_number = house_number(self.text(lo)).is_some();
        // "3739 COUNTY ROAD 8 LAUREL": a highway route ends the street
        // (section 237), or a suffix after a name past the route
        // ("HWY 66 FRONTAGE RD TULSA").
        let mut start = if first_number { lo + 1 } else { lo };
        if start < hi && is_fraction(self.text(start)) {
            start += 1;
        }
        if let Some(h) = self.highway_at(start, hi) {
            let c = h.route + 1;
            let p = (c + 1..hi).find(|&k| is_suffix(k) && k + 1 < hi && !is_suffix(k + 1));
            return self.after_street(p.map_or(c, |p| p + 1), hi);
        }
        // The street ends at a suffix with a name before it. Prefer the
        // first one followed by a word that is not also a suffix
        // ("12 LAKE SHORE DR CHICAGO"), else the last one.
        let min = if first_number { lo + 2 } else { lo + 1 };
        let suffix = (min..hi)
            .find(|&k| is_suffix(k) && k + 1 < hi && !is_suffix(k + 1))
            .or_else(|| (min..hi).rev().find(|&k| is_suffix(k)));
        let Some(p) = suffix else {
            let digits = self.toks[lo..hi].iter().any(|t| has_digit(&t.text));
            return (!digits).then_some(lo);
        };
        self.after_street(p + 1, hi)
    }

    /// Where the city starts once the street's name ends before `c`: after
    /// a postdirectional and a unit.
    fn after_street(&self, c: usize, hi: usize) -> Option<usize> {
        let mut c = c;
        if c < hi && self.lex.directional(self.text(c)).is_some() {
            // A directional with nothing after it is the postdirectional
            // (`HIGHWAY 29 SOUTH 32533`), not a city.
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
        // Line 2 `CAPE CORAL` with city `CAPE CORAL`: a whole line or comma
        // segment that is the city, even when its first word is a suffix.
        let k = words.len();
        if hi > lo + k {
            let tail = &self.toks[hi - k..hi];
            let whole_seg = self.toks[hi - k - 1].seg != tail[0].seg
                && tail.iter().all(|t| t.seg == tail[0].seg)
                && !self.toks.get(hi).is_some_and(|t| t.seg == tail[0].seg);
            if whole_seg && tail.iter().zip(&words).all(|(t, w)| t.text == *w) {
                self.set(hi - k..hi, Tag::Skip);
                return hi - k;
            }
        }
        for k in (1..=words.len()).rev() {
            if hi < lo + k + 1 {
                continue;
            }
            let tail = &self.toks[hi - k..hi];
            // "100 MAIN ST" before "ST PETERSBURG": a suffix is the street's.
            if self.lex.suffix(&tail[0].text).is_some() {
                continue;
            }
            // "SUITE A" before "ARCHER": a value after a designator or '#'
            // is the unit's.
            let before = &self.toks[hi - k - 1];
            if before.is_hash() || self.lex.unit(&before.text).is_some() {
                continue;
            }
            // One cut-off word needs two letters or more; a lone letter is
            // more likely a unit ("DR B") than a copy of "BRADENTON".
            if k == 1 && tail[0].text.len() < 2 && tail[0].text != words[0] {
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

    /// A ZIP ending the address line that repeats the ZIP already tagged on
    /// the last line (`413 OLD CORRY FIELD RD B 32506` before
    /// `PENSACOLA, FL- 32506`, Escambia). The copy is skipped. Returns the
    /// new end of the line.
    pub(super) fn drop_repeated_zip(&mut self, lo: usize, hi: usize) -> usize {
        if hi < lo + 2 || zip_parts(self.text(hi - 1)).is_none() || self.toks[hi - 2].is_hash() {
            return hi;
        }
        let copy = self.text(hi - 1);
        let repeated = (0..self.toks.len())
            .any(|i| i != hi - 1 && self.tags[i] == Tag::Zip && self.text(i) == copy);
        if repeated {
            self.set(hi - 1..hi, Tag::Skip);
            return hi - 1;
        }
        hi
    }

    /// A state ending the address line that repeats the state already
    /// tagged from its field (`4486 W OAKLAWN ST LECANTO FL` with state
    /// `FL`). The copy is skipped, so the city copy before it can be too.
    /// Returns the new end of the line.
    pub(super) fn drop_repeated_state(&mut self, lo: usize, hi: usize) -> usize {
        let code = |s: &str| self.lex.state(s).map(|l| l.value.code());
        let field: Vec<_> = (0..self.toks.len())
            .filter(|&i| self.tags[i] == Tag::State)
            .map(|i| self.toks[i].clone())
            .collect();
        let Some(tagged) = code(&join(&field)) else {
            return hi;
        };
        match longest_ending(self.toks, lo + 1, hi, |s| self.state_ok(s, false)) {
            Some((_, w)) if code(&join(&self.toks[hi - w..hi])) == Some(tagged) => {
                self.set(hi - w..hi, Tag::Skip);
                hi - w
            }
            _ => hi,
        }
    }

    // ---- delivery line -----------------------------------------------------

    pub(super) fn delivery(&mut self, lo: usize, hi: usize) {
        let (lo, hi) = self.attention(lo, hi);
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
        // "PO BOX 12442 PO BOX 12442": the same line written twice.
        let n = hi - lo;
        if n >= 2 && n % 2 == 0 {
            let half = n / 2;
            let twice = (0..half).all(|k| self.text(lo + k) == self.text(lo + half + k));
            if twice {
                self.set(lo + half..hi, Tag::Skip);
                return self.delivery(lo, lo + half);
            }
        }
        let hi = self.building(lo, hi);
        if lo >= hi || self.dual(lo, hi) || self.special_line(lo, hi) {
            return;
        }
        self.street(lo, hi);
    }

    /// A building name at the end of the delivery lines, after the street
    /// or its unit: words ending in a `[building]` word, in their own line
    /// or comma segment (`..., EAST TOWER`, line 2 `ANNEX BUILDING`) or
    /// right after a unit (`SUITE 801 EAST TOWER`). Returns the new end.
    fn building(&mut self, lo: usize, hi: usize) -> usize {
        if hi < lo + 3 || !self.lex.is_building_word(self.text(hi - 1)) {
            return hi;
        }
        let seg = self.toks[hi - 1].seg;
        let mut k = hi - 1;
        while k > lo && self.toks[k - 1].seg == seg {
            let t = self.text(k - 1);
            if has_digit(t) || self.toks[k - 1].is_hash() || self.lex.unit(t).is_some() {
                break;
            }
            k -= 1;
        }
        if hi - k < 2 || k == lo {
            return hi;
        }
        let starts_segment = self.toks[k - 1].seg != seg;
        let after_unit = has_digit(self.text(k - 1))
            && k >= lo + 2
            && (self.toks[k - 2].is_hash() || self.lex.unit(self.text(k - 2)).is_some());
        if !(starts_segment || after_unit) {
            return hi;
        }
        self.set(k..hi, Tag::Building);
        k
    }

    /// Pub 28 section 214: an attention or care-of line (`ATTN LAW DEPT`,
    /// `C/O YMF`), from the ATTN or C/O word to the end of its line or comma
    /// segment, at the start or the end of the delivery lines. Returns the
    /// range left for the address.
    fn attention(&mut self, lo: usize, hi: usize) -> (usize, usize) {
        let lex = self.lex;
        let word = |k: usize| {
            longest_at(self.toks, k, hi, |s| {
                lex.special(s).filter(|l| l.value == Special::Attention)
            })
            .map(|(_, w)| w)
        };
        let Some((k, w)) = (lo..hi).find_map(|k| word(k).map(|w| (k, w))) else {
            return (lo, hi);
        };
        // "C/O 2775 NW 49TH AVE": the address itself follows; only the word
        // is dropped.
        if k + w < hi && house_number(self.text(k + w)).is_some() {
            self.set(k..k + w, Tag::Skip);
            return if k == lo { (k + w, hi) } else { (lo, hi) };
        }
        let end = (k..hi)
            .find(|&e| self.toks[e].seg != self.toks[k].seg)
            .unwrap_or(hi);
        if end == hi {
            // "PO BOX 23181 C/O YMF", or line 2 "ATTN: RYAN ADAIR".
            self.set(k..hi, Tag::Attention);
            (lo, k)
        } else if k == lo {
            // Line 1 "C/O BRADY SULLIVAN PROPERTIES", the street on line 2.
            self.set(lo..end, Tag::Attention);
            (end, hi)
        } else {
            (lo, hi)
        }
    }

    /// Pub 28 section 215: a street address and a PO Box in one record.
    /// Both are tagged and [`Warning::DualAddress`] is raised. The box may
    /// come first (`PO BOX 286` / `23320 NE HWY 315`) or after the street
    /// (`9418 CUBA RD` / `P.O. BOX 100`, `... AVE PO BOX 365`); the street
    /// needs a house number. A bare `BOX 88` counts only when it starts a
    /// later line or comma segment.
    /// Returns whether the line was one.
    fn dual(&mut self, lo: usize, hi: usize) -> bool {
        if let Some(m) = self.box_line(lo, hi, false) {
            let rest = m.end;
            if rest >= hi || self.starts_unit(rest, hi) {
                return false;
            }
            // The street needs its house number: `PO BOX 3163` / `GRAND
            // CENTRAL STATION` is a box with a postal station, not a street.
            if !(rest + 1 < hi && house_number(self.text(rest)).is_some()) {
                return false;
            }
            for (r, tag) in m.tags {
                self.set(r, tag);
            }
            self.street(rest, hi);
            self.warn(Warning::DualAddress);
            return true;
        }
        if hi < lo + 3 || house_number(self.text(lo)).is_none() {
            return false;
        }
        for k in lo + 2..hi {
            let starts_segment = self.toks[k].seg != self.toks[k - 1].seg;
            let Some(m) = self.box_line(k, hi, starts_segment) else {
                continue;
            };
            for (r, tag) in m.tags {
                self.set(r, tag);
            }
            self.units(m.end, m.end, hi, false);
            self.street(lo, k);
            self.warn(Warning::DualAddress);
            return true;
        }
        false
    }

    /// A PO Box (`PO BOX 12`, `DRAWER 9`, or with `bare` a plain `BOX 88`)
    /// and its value at `k`, without tagging.
    fn box_line(&self, k: usize, hi: usize, bare: bool) -> Option<SpecialMatch> {
        let (l, w) = longest_at(self.toks, k, hi, |s| self.lex.special(s))?;
        if !(l.value == Special::PoBox || bare && l.value == Special::Box) {
            return None;
        }
        let (v, hash) = self.value_at(k + w, hi)?;
        let mut tags = vec![(k..k + w, Tag::PoBox)];
        if hash {
            tags.push((v - 1..v, Tag::Skip));
        }
        tags.push((v..v + 1, Tag::PoBoxNumber));
        Some(SpecialMatch {
            tags,
            end: v + 1,
            units: true,
        })
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
                Special::Box => {
                    if let Some((v, hash)) = self.value_at(lo + w, hi) {
                        // Section 238.1 writes PSC 103 BOX 2877; the reverse
                        // (`BOX 2877 PSC 103`) is read the same way.
                        if let Some((_, mw)) = longest_at(self.toks, v + 1, hi, |s| lex.military(s))
                        {
                            if let Some((num, nhash)) = self.value_at(v + 1 + mw, hi) {
                                tags.push((lo..lo + w, Tag::Skip));
                                value(&mut tags, v, hash, Tag::MilitaryBoxNumber);
                                tags.push((v + 1..v + 1 + mw, Tag::MilitaryKind));
                                value(&mut tags, num, nhash, Tag::MilitaryNumber);
                                return Some(SpecialMatch {
                                    tags,
                                    end: num + 1,
                                    units: false,
                                });
                            }
                        }
                        // `BOX 123456` alone on the line is a PO Box.
                        if v + 1 == hi {
                            tags.push((lo..lo + w, Tag::PoBox));
                            value(&mut tags, v, hash, Tag::PoBoxNumber);
                            return Some(SpecialMatch {
                                tags,
                                end: hi,
                                units: false,
                            });
                        }
                    }
                }
                Special::Attention => {}
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
        // "APT 1B 626 E KILBOURN AVE": a unit before the street. A unit
        // found after the street becomes the second one.
        let Some(start) = self.leading_unit(lo, hi) else {
            return self.street_from(lo, hi);
        };
        self.street_from(start, hi);
        for t in &mut self.tags[start..hi] {
            match *t {
                Tag::UnitType(0) => *t = Tag::UnitType(1),
                Tag::UnitNumber(0) => *t = Tag::UnitNumber(1),
                _ => {}
            }
        }
    }

    /// A unit (`APT 1B`, `# 5`) at `lo` followed by a house number: tags it
    /// and returns where the street starts.
    fn leading_unit(&mut self, lo: usize, hi: usize) -> Option<usize> {
        let (ty, k) = if self.toks[lo].is_hash() {
            (lo..lo, lo)
        } else {
            let (_, w) = longest_at(self.toks, lo, hi, |s| self.lex.unit(s))?;
            (lo..lo + w, lo + w)
        };
        let (v, hash) = self.value_at(k, hi)?;
        let start = v + 1;
        if start + 1 >= hi || house_number(self.text(start)).is_none() {
            return None;
        }
        self.set(ty, Tag::UnitType(0));
        if hash {
            self.set(v - 1..v, Tag::Skip);
        }
        self.set(v..v + 1, Tag::UnitNumber(0));
        Some(start)
    }

    fn street_from(&mut self, lo: usize, hi: usize) {
        let mut i = lo;
        let t0 = self.text(i);
        if is_glued_ordinal(t0) {
            // "409742ND SQ": 4097 42ND or 40974 2ND. Don't guess.
            self.warn(Warning::AmbiguousStreetNumber);
            self.set(lo..hi, Tag::Unparsed);
            return;
        }
        // "31967,32037,32067, PUNTA GORDA": a list of house numbers, each in
        // its own comma segment.
        let numbers = (lo..hi)
            .take_while(|&k| {
                all_digits(self.text(k)) && (k == lo || self.toks[k].seg != self.toks[k - 1].seg)
            })
            .count();
        if numbers >= 2 {
            self.warn(Warning::MultipleAddresses);
            self.set(lo..lo + numbers, Tag::Unparsed);
            i += numbers;
        } else if is_slash_pair(t0) {
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
            } else if i < hi && is_grid_number(t0) && is_grid_number(self.text(i)) {
                // "N165 W2123": a grid number written in two parts.
                self.set(i..i + 1, Tag::StreetNumber);
                i += 1;
            } else if i + 1 < hi && self.number_letter(i) {
                // "428 B SOUTH MAIN ST": the letter belongs to the number.
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
            self.set(h.state.clone(), Tag::HighwayState);
            self.set(h.prefix.clone(), Tag::HighwayPrefix);
            self.set(h.route..h.route + 1, Tag::HighwayRoute);
            // Words after the route are the street's only on the route's
            // line or comma segment; the rest is left unparsed.
            let seg = self.toks[h.route].seg;
            let line_end = (floor..j).find(|&k| self.toks[k].seg != seg).unwrap_or(j);
            self.street_tail(floor, line_end);
            return;
        }

        let j = self.units(i, i, hi, true);
        // The name and suffix come from the first line; whatever is left of
        // a later line ("..., BLDG 5 ROOM") is not part of the name.
        let mut line_end = (i..j)
            .find(|&k| self.toks[k].seg != self.toks[i].seg)
            .unwrap_or(j);
        // "2272 GOODWOOD BLVD., SE": a postdirectional after a comma.
        let rest = line_end..j;
        if !rest.is_empty()
            && self.toks[rest.clone()]
                .iter()
                .all(|t| t.seg == self.toks[rest.start].seg)
            && longest_at(self.toks, rest.start, j, |s| self.lex.directional(s))
                .is_some_and(|(_, w)| w == rest.len())
        {
            line_end = j;
        }
        if self.street_continues(i, line_end, j) {
            line_end = j;
        }
        self.street_body(i, line_end, hi.min(line_end));
    }

    /// A street cut across lines (`8725 NW` / `18 TER`, `5424 EDGEWATER` /
    /// `AVE`): the first line's part `i..e` has no suffix, and the rest
    /// `e..j`, one line or comma segment, is a suffix with at most a number
    /// before it and a directional after it.
    fn street_continues(&self, i: usize, e: usize, j: usize) -> bool {
        if i >= e || e >= j || self.lex.suffix(self.text(e - 1)).is_some() {
            return false;
        }
        let rest = &self.toks[e..j];
        if rest.iter().any(|t| t.seg != rest[0].seg) || rest[0].seg == self.toks[e - 1].seg {
            return false;
        }
        let mut k = j;
        if let Some((_, w)) = longest_ending(self.toks, e, k, |s| self.lex.directional(s)) {
            k -= w;
        }
        let is_number = |t: &str| all_digits(t) || is_ordinal(t);
        match k - e {
            1 => self.lex.suffix(self.text(e)).is_some(),
            2 => is_number(self.text(e)) && self.lex.suffix(self.text(e + 1)).is_some(),
            _ => false,
        }
    }

    /// Whether the single letter at `k` belongs to the house number before
    /// it (`428 B SOUTH MAIN ST`): not a directional itself (`123 N MAIN`),
    /// and followed by a directional. A name word after it would make the
    /// letter an initial (`820 A PHILIP RANDOLPH BLVD`, `9058 T BIRD RD`).
    fn number_letter(&self, k: usize) -> bool {
        let t = self.text(k);
        is_letter(t)
            && self.lex.directional(t).is_none()
            && self.lex.directional(self.text(k + 1)).is_some()
    }

    /// Whether the tokens in `r` are the penthouse designator.
    fn is_penthouse(&self, r: &Range<usize>) -> bool {
        r.len() == 1
            && self
                .lex
                .unit(self.text(r.start))
                .is_some_and(|u| u.value.abbr() == "PH")
    }

    /// Pub 28 section 237 and Appendix F: a highway prefix and route right
    /// after the house number (and an optional predirectional). Appendix F
    /// also names a state before the prefix (`KY STATE HIGHWAY 1`), or before
    /// the route alone (`KENTUCKY 440`); those are tried last, so `NE HWY 5`
    /// keeps NE as the predirectional.
    fn highway_at(&self, i: usize, hi: usize) -> Option<HighwayAt> {
        let lex = self.lex;
        // A state pack may add prefix spellings (`SR` in Florida).
        let state = self.places.map(|c| c.state);
        let try_at = |p: usize| {
            let (l, w) = longest_at(self.toks, p, hi, |s| match state {
                Some(st) => lex.highway_in(st, s),
                None => lex.highway(s),
            })?;
            let route = p + w;
            // Appendix F allows letter routes (`STATE ROUTE MM`); taken only
            // after a county prefix (`COUNTY ROAD J`), where a letter is
            // otherwise easy to read as a unit.
            let county = matches!(l.value.standard(), "COUNTY ROAD" | "COUNTY HIGHWAY");
            let letters = |t: &str| t.len() <= 2 && t.bytes().all(|c| c.is_ascii_alphabetic());
            let ok = route < hi
                && (is_route_value(self.text(route)) || county && letters(self.text(route)));
            ok.then_some((p..p + w, route))
        };
        if let Some((prefix, route)) = try_at(i) {
            return Some(HighwayAt {
                pre: i..i,
                state: i..i,
                prefix,
                route,
            });
        }
        if let Some((_, w)) = longest_at(self.toks, i, hi, |s| lex.directional(s)) {
            if let Some((prefix, route)) = try_at(i + w) {
                return Some(HighwayAt {
                    pre: i..i + w,
                    state: i + w..i + w,
                    prefix,
                    route,
                });
            }
        }
        let (_, w) = longest_at(self.toks, i, hi, |s| self.highway_state(s))?;
        let state = i..i + w;
        if let Some((prefix, route)) = try_at(i + w) {
            return Some(HighwayAt {
                pre: i..i,
                state,
                prefix,
                route,
            });
        }
        // Bare `KY 1207`: digits only, and not a directional that happens to
        // be a state code (`NE 5TH ST`). A state code that is also a
        // designator counts too (`5100 FL 40 SUITE 100`): this is the start of
        // the street, and read as a floor it would leave no street name.
        let route = i + w;
        let word = join(&self.toks[state.clone()]);
        let bare = route < hi && all_digits(self.text(route)) && lex.directional(&word).is_none();
        bare.then_some(HighwayAt {
            pre: i..i,
            state,
            prefix: route..route,
            route,
        })
    }

    /// A state code or full name (not a traditional abbreviation such as
    /// KENT, nor a military code) inside a highway name.
    fn highway_state(&self, s: &str) -> Option<()> {
        let l = self.lex.state(s)?;
        (l.kind != MatchKind::Variant && l.value.kind() != StateKind::Military).then_some(())
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

    /// Whether tokens `last - 1` and `last` are one unit value written in two
    /// parts, a letter and a number in either order, right after a designator
    /// or `#`: `APT E 1003`, `UNIT 210 C`, and `# V 21`, the form the USPS
    /// emitter writes for a bare `DR V 21`.
    fn split_value(&self, floor: usize, last: usize) -> bool {
        if last < floor + 2 {
            return false;
        }
        let (a, b) = (self.text(last - 1), self.text(last));
        let pair = is_letter(a) && all_digits(b) || all_digits(a) && is_letter(b);
        let v = last - 1;
        let seg = self.toks[last].seg;
        let one_seg = self.toks[v].seg == seg && self.toks[v - 1].seg == seg;
        pair && one_seg
            && (self.toks[v - 1].is_hash()
                || longest_ending(self.toks, floor, v, |s| self.lex.unit(s)).is_some())
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
            // "STE 1500" before " - ": a dash between the street and its
            // unit (`3RD AVENUE - SUITE 1500`).
            if !found.is_empty() && self.toks[last].is_dash() {
                self.set(last..j, Tag::Skip);
                j = last;
                continue;
            }
            // "# 309", "UNIT 203", "LOT H28".
            if is_unit_value(t) && last > floor {
                let start = if self.split_value(floor, last) {
                    last - 1
                } else {
                    last
                };
                if self.toks[start - 1].is_hash() {
                    self.set(start - 1..start, Tag::Skip);
                    // "UNIT # 4": the designator before the '#' is this unit's.
                    let hash = start - 1;
                    match longest_ending(self.toks, floor, hash, |s| lex.unit(s)) {
                        Some((_, w)) => {
                            found.push((hash - w..hash, start..j));
                            j = hash - w;
                        }
                        None => {
                            found.push((hash..hash, start..j));
                            j = hash;
                        }
                    }
                    continue;
                }
                if let Some((_, w)) = longest_ending(self.toks, floor, start, |s| lex.unit(s)) {
                    found.push((start - w..start, start..j));
                    j = start - w;
                    continue;
                }
            }
            // A later line or comma segment that is only a number: "403" as
            // line 2 of `2309 1ST AVE NO`. A unit with no known designator.
            // A name of two words or more must come first: `1921 NW` / `46`
            // is a street cut across lines, not NW with unit 46.
            let alone = last > name_lo + 1
                && self.toks[last].seg != self.toks[last - 1].seg
                && (last + 1 == self.toks.len() || self.toks[last + 1].seg != self.toks[last].seg);
            if found.is_empty() && j == hi && alone && all_digits(t) {
                found.push((last..last, last..j));
                j = last;
                continue;
            }
            // An ordinal before the designator: "3RD FLOOR".
            if let Some((_, w)) = longest_ending(self.toks, floor + 1, j, |s| lex.unit(s)) {
                let v = j - w - 1;
                if v > name_lo && is_ordinal(self.text(v)) {
                    found.push((j - w..j, v..v + 1));
                    j = v;
                    continue;
                }
            }
            // A designator with nothing after it: "... TRL UNIT".
            if let Some((u, w)) = longest_ending(self.toks, floor + 1, j, |s| lex.unit(s)) {
                if j - w > name_lo + 1 {
                    // A penthouse right after a designator is its number
                    // (`APT PH 5213`, `SUITE PH`; design.md decision 31).
                    if let Some((ty, num)) = found.last_mut() {
                        if u.value.abbr() != "PH" && ty.start == j && self.is_penthouse(ty) {
                            let end = if num.start == num.end {
                                ty.end
                            } else {
                                num.end
                            };
                            *num = ty.start..end;
                            *ty = j - w..j;
                            j -= w;
                            continue;
                        }
                    }
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
