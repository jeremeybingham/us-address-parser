//! Tokenizer (pipeline step 2).
//!
//! Splits cleaned text into tokens that keep their byte span in the
//! original input. Commas and semicolons separate *segments* (address lines
//! or parts of one), which the tagger uses as strong boundaries.
//!
//! Punctuation rules, from Pub 28 and the shapes seen in county address data:
//! - `#` is always its own token: `#309` -> `#` `309`.
//! - Parentheses are dropped from the text but kept in the span, and the
//!   token is marked `paren`: `(110)` -> `110`.
//! - A hyphen alone is a separator token (`N/A - MAGNOLIA AVE`). Leading and
//!   trailing hyphens are dropped (`-UNIT`, `FL-`); inner ones are kept
//!   (`C-22`, `1204-1206`).
//! - Periods are dropped except between digits (`ST.` -> `ST`, `39.2`). A
//!   period after an abbreviation of two letters or more, with letters right
//!   after it, separates words: `ST.CLOUD` -> `ST` `CLOUD` (initials such as
//!   `P.O.` and `U.S.A.` stay joined).
//! - Trailing colons and quotes are dropped.
//!
//! Glued tokens are split when the pieces are unambiguous:
//! - house number run into a name: `101BRADDOCK` -> `101` `BRADDOCK`, but
//!   not an ordinal (`42ND`) or a one-letter suffix (`12A`);
//! - a number run into the suffix `ST` or `RD` when that is not the number's
//!   ordinal ending: `178ST` -> `178` `ST` (178's ordinal is `178TH`), but
//!   `21ST` and `3RD` stay whole;
//! - a name run into a ZIP: `TAMPA33616` -> `TAMPA` `33616`;
//! - a highway prefix run into its route: `I10`, `IH280` (Pub 28 Appendix F);
//! - a unit designator run into its number: `APT107`, `APT3D`, or joined by
//!   punctuation: `APT.1285`, `STE.200`, `APT:201`, `UNIT-4`;
//! - a street suffix run into a unit designator, before a number:
//!   `CTAPT 107` -> `CT` `APT` `107`;
//! - a street suffix run into a city a state pack knows, when the city ends
//!   the segment or comes before a state or ZIP:
//!   `WILSON STREETDADE CITY FL` -> `STREET` `DADE` `CITY`.

use crate::classify::{is_ordinal, zip_parts, ORDINAL_SUFFIXES};
use crate::clean::Cleaned;
use crate::lexicon::{Lexicons, MatchKind, Special, MAX_PLACE_WORDS};
use crate::types::{FieldName, Span};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Token {
    /// Normalized text: uppercase, punctuation handled as above.
    pub text: String,
    /// Span in the original input, including any stripped punctuation.
    pub span: Span,
    /// Comma-separated segment index, counted across the whole input.
    pub seg: usize,
    /// Written inside parentheses.
    pub paren: bool,
}

impl Token {
    pub fn is_dash(&self) -> bool {
        self.text == "-"
    }

    pub fn is_hash(&self) -> bool {
        self.text == "#"
    }
}

/// Tokenize one input string (a single line, or one field). `seg0` is the
/// first segment index to use, so fields can share one numbering.
pub(crate) fn tokenize(
    cleaned: &Cleaned,
    field: Option<FieldName>,
    seg0: usize,
    lex: &Lexicons,
) -> Vec<Token> {
    let text = cleaned.text.as_str();
    let bytes = text.as_bytes();
    // Raw pieces: (start, end, segment) in cleaned byte offsets.
    let mut pieces = Vec::new();
    let mut seg = seg0;
    let mut start = None;
    for (i, &b) in bytes.iter().enumerate() {
        let sep = b == b' ' || b == b',' || b == b';';
        if sep {
            if let Some(s) = start.take() {
                pieces.push((s, i, seg));
            }
            if b != b' ' {
                seg += 1;
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if let Some(s) = start {
        pieces.push((s, bytes.len(), seg));
    }

    let mut out = Vec::new();
    let mut in_paren = false;
    for (k, &(a, b, seg)) in pieces.iter().enumerate() {
        let next_starts_numeric = pieces.get(k + 1).is_some_and(|&(na, _, nseg)| {
            nseg == seg
                && text[na..]
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_digit() || c == '#')
        });
        // The words after this piece in its segment, for a city glued to
        // the street.
        let after: Vec<&str> = pieces[k + 1..]
            .iter()
            .take_while(|&&(_, _, s)| s == seg)
            .map(|&(na, nb, _)| text[na..nb].trim_matches(|c: char| !c.is_alphanumeric()))
            .collect();
        let later = || {
            pieces[k + 1..]
                .iter()
                .filter(|&&(_, _, s)| s > seg)
                .map(|&(na, nb, _)| text[na..nb].trim_matches(|c: char| !c.is_alphanumeric()))
        };
        // The next segment starts with a state or a ZIP: the glued city's end.
        let later_seg = later()
            .next()
            .is_some_and(|w| lex.state(w).is_some() || zip_parts(w).is_some());
        let later_anchor = later().any(|w| lex.state(w).is_some() || zip_parts(w).is_some());
        let mut p = Piece {
            cleaned,
            field,
            seg,
            after: &after,
            later_seg,
            later_anchor,
            out: &mut out,
        };
        let raw = &text[a..b];
        let opens = raw.starts_with('(');
        let closes = raw.ends_with(')');
        let paren = in_paren || opens;
        if opens {
            in_paren = true;
        }
        if closes {
            in_paren = false;
        }
        p.piece(a, b, paren, next_starts_numeric, lex);
    }
    out
}

struct Piece<'a> {
    cleaned: &'a Cleaned,
    field: Option<FieldName>,
    seg: usize,
    /// The words after this piece in its segment.
    after: &'a [&'a str],
    /// Whether another segment follows this one.
    later_seg: bool,
    /// Whether a later segment holds a state or a ZIP.
    later_anchor: bool,
    out: &'a mut Vec<Token>,
}

impl Piece<'_> {
    /// The words before this piece in its segment, nearest last.
    fn before(&self) -> Vec<&str> {
        let mut words: Vec<&str> = self
            .out
            .iter()
            .rev()
            .take_while(|t| t.seg == self.seg)
            .take(MAX_PLACE_WORDS)
            .map(|t| t.text.as_str())
            .collect();
        words.reverse();
        words
    }

    fn span(&self, a: usize, b: usize) -> Option<Span> {
        let (start, end) = self.cleaned.original_range(a, b)?;
        Some(Span {
            field: self.field,
            start,
            end,
        })
    }

    fn emit(&mut self, text: String, a: usize, b: usize, paren: bool) {
        if text.is_empty() {
            return;
        }
        if let Some(span) = self.span(a, b) {
            self.out.push(Token {
                text,
                span,
                seg: self.seg,
                paren,
            });
        }
    }

    /// One whitespace/comma-delimited piece, `cleaned.text[a..b]`.
    fn piece(&mut self, a: usize, b: usize, paren: bool, next_numeric: bool, lex: &Lexicons) {
        let text = &self.cleaned.text;
        let raw = &text[a..b];
        if raw.bytes().all(|c| c == b'-') {
            self.emit("-".to_string(), a, b, paren);
            return;
        }
        // A line of only asterisks stays, so it reads as a placeholder.
        if raw.bytes().all(|c| c == b'*') {
            self.emit("*".to_string(), a, b, paren);
            return;
        }
        // Trim wrapping punctuation from the text, but keep the whole piece
        // in the span when it is a single token.
        // Asterisks too: Pub 28 section 354 drops them (`**Private Address**`),
        // and a stray backtick (`1719\``).
        let trim: &[char] = &['(', ')', '-', ':', '"', '\'', '.', '[', ']', '*', '`'];
        let lead = raw.len() - raw.trim_start_matches(trim).len();
        let inner = raw.trim_matches(trim);
        if inner.is_empty() {
            return;
        }
        let ia = a + lead;
        let ib = ia + inner.len();
        // Split around '#'.
        if inner.contains('#') {
            let mut s = ia;
            for (i, c) in text[ia..ib].char_indices() {
                if c == '#' {
                    self.sub(s, ia + i, a, b, paren, next_numeric, lex);
                    self.emit("#".to_string(), ia + i, ia + i + 1, paren);
                    s = ia + i + 1;
                }
            }
            self.sub(s, ib, a, b, paren, next_numeric, lex);
        } else {
            self.sub(ia, ib, a, b, paren, next_numeric, lex);
        }
    }

    /// A sub-piece `[s, e)` of the piece `[pa, pb)`. When it is the whole
    /// trimmed piece, the span covers the untrimmed piece.
    #[allow(clippy::too_many_arguments)]
    fn sub(
        &mut self,
        s: usize,
        e: usize,
        pa: usize,
        pb: usize,
        paren: bool,
        next_numeric: bool,
        lex: &Lexicons,
    ) {
        if s >= e {
            return;
        }
        let t = &self.cleaned.text[s..e];
        let t = t.trim_matches(|c| c == '-' || c == '.');
        if t.is_empty() {
            return;
        }
        let s2 = s + self.cleaned.text[s..e].find(t).unwrap_or(0);
        let e2 = s2 + t.len();
        let whole = self.cleaned.text[pa..pb].trim_matches(|c: char| !c.is_alphanumeric()) == t
            && !self.cleaned.text[pa..pb].contains('#');
        let (sa, sb) = if whole { (pa, pb) } else { (s2, e2) };

        // A hyphenated route: `GA-25`, `US-385`, `CR-48`, `I-65` (Appendix F
        // writes these with a space). Only after a highway prefix or a state
        // code that is not also a directional (`NE-2` stays whole); SR is
        // left to state packs.
        if let Some(cut) = route_hyphen(t, lex) {
            self.emit(t[..cut].to_string(), s2, s2 + cut, paren);
            self.emit(t[cut + 1..].to_string(), s2 + cut + 1, e2, paren);
            return;
        }
        // A unit designator joined to its value by punctuation: `APT.1285`,
        // `STE.200`, `APT:201`, `UNIT-4`.
        if let Some(cut) = unit_punct(t, lex) {
            self.emit(t[..cut].to_string(), s2, s2 + cut, paren);
            self.sub(s2 + cut + 1, e2, pa, pb, paren, next_numeric, lex);
            return;
        }
        // `ST.CLOUD`, `FT.MYERS`: a period after two letters or more, with a
        // letter after it, ends a word.
        if let Some(cut) =
            word_period(t).filter(|&c| !known_word(&format!("{}{}", &t[..c], &t[c + 1..]), lex))
        {
            self.sub(s2, s2 + cut, pa, pb, paren, next_numeric, lex);
            self.sub(s2 + cut + 1, e2, pa, pb, paren, next_numeric, lex);
            return;
        }
        // A value right after '#' or a unit designator is that unit's: `# 2WD`
        // is not house number 2 and a name, `APT I4` is not highway I4.
        let unit_value = self
            .out
            .last()
            .is_some_and(|p| p.seg == self.seg && (p.is_hash() || lex.unit(&p.text).is_some()));
        // A route right after a highway prefix may end in letters
        // (Appendix F: `ROUTE 1150EE`).
        let after_prefix = self
            .out
            .last()
            .is_some_and(|p| p.seg == self.seg && lex.highway(&p.text).is_some());
        if t.bytes().all(|c| c.is_ascii_alphanumeric()) {
            let cut = glue_split(t, next_numeric, unit_value, after_prefix, lex).or_else(|| {
                glue_place(
                    t,
                    self.before(),
                    self.after,
                    self.later_seg,
                    self.later_anchor,
                    lex,
                )
            });
            if let Some(cut) = cut {
                self.emit(t[..cut].to_string(), s2, s2 + cut, paren);
                self.emit(t[cut..].to_string(), s2 + cut, e2, paren);
                return;
            }
        }
        self.emit(normalize(t), sa, sb, paren);
    }
}

/// Drop periods except between digits; drop quotes.
fn normalize(t: &str) -> String {
    let b = t.as_bytes();
    let mut out = String::with_capacity(t.len());
    for (i, c) in t.char_indices() {
        match c {
            '.' => {
                let digit_before = i > 0 && b[i - 1].is_ascii_digit();
                let digit_after = b.get(i + 1).is_some_and(u8::is_ascii_digit);
                if digit_before && digit_after {
                    out.push('.');
                }
            }
            '"' | '\'' => {}
            _ => out.push(c),
        }
    }
    out
}

/// The hyphen's index in a hyphenated route such as `GA-25`, if `t` is one.
fn route_hyphen(t: &str, lex: &Lexicons) -> Option<usize> {
    let (letters, route) = t.split_once('-')?;
    let ok_letters = !letters.is_empty()
        && letters.len() <= 3
        && letters.bytes().all(|c| c.is_ascii_alphabetic());
    let ok_route = route.bytes().next().is_some_and(|c| c.is_ascii_digit())
        && route.bytes().all(|c| c.is_ascii_alphanumeric());
    if !(ok_letters && ok_route) {
        return None;
    }
    let state = lex
        .state(letters)
        .is_some_and(|l| l.kind != MatchKind::Variant)
        && lex.directional(letters).is_none();
    (lex.highway(letters).is_some() || state).then_some(letters.len())
}

/// The index of the first period in `t` that has two letters or more right
/// before it (back to the start or another period) and a letter right after.
/// The caller keeps the token whole when the pieces join into a known word
/// (`BLV.D` is `BLVD`).
fn word_period(t: &str) -> Option<usize> {
    let b = t.as_bytes();
    let mut start = 0;
    for (i, &c) in b.iter().enumerate() {
        if c != b'.' {
            continue;
        }
        let before = &b[start..i];
        if before.len() >= 2
            && before.iter().all(u8::is_ascii_alphabetic)
            && b.get(i + 1).is_some_and(u8::is_ascii_alphabetic)
        {
            return Some(i);
        }
        start = i + 1;
    }
    None
}

/// The index of the punctuation in `APT.1285`, `STE.200`, `APT:201`,
/// `UNIT-4`, `BOX.2351`: a unit designator or box word, one of `.` `:` `-`,
/// then the value.
fn unit_punct(t: &str, lex: &Lexicons) -> Option<usize> {
    let cut = t.find(['.', ':', '-'])?;
    let (letters, rest) = (&t[..cut], &t[cut + 1..]);
    let ok = letters.len() >= 2
        && letters.bytes().all(|c| c.is_ascii_alphabetic())
        && rest
            .bytes()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric())
        && (lex.unit(letters).is_some()
            || lex
                .special(letters)
                .is_some_and(|l| matches!(l.value, Special::Box | Special::PoBox)));
    ok.then_some(cut)
}

/// Where to split a glued alphanumeric token, if anywhere.
fn glue_split(
    t: &str,
    next_numeric: bool,
    after_unit: bool,
    after_prefix: bool,
    lex: &Lexicons,
) -> Option<usize> {
    let digits_lead = t.bytes().take_while(u8::is_ascii_digit).count();
    let alpha_lead = t.bytes().take_while(u8::is_ascii_alphabetic).count();
    let n = t.len();
    // A value right after a unit designator is whole (`# 2WD`, `APT I4`).
    if after_unit && (digits_lead > 0 || alpha_lead < n) {
        return None;
    }
    if digits_lead > 0 && digits_lead < n {
        let rest = &t[digits_lead..];
        // 101BRADDOCK; not 12A, 42ND, 1ST.
        // A route after a highway prefix keeps up to two letters (`1150EE`).
        if rest.len() >= 2
            && !(after_prefix && rest.len() <= 2)
            && rest.bytes().all(|c| c.is_ascii_alphabetic())
            && !ORDINAL_SUFFIXES.contains(&rest)
            && !is_ordinal(t)
        {
            return Some(digits_lead);
        }
        // 178ST, 54RD: an ordinal ending the number does not take is the
        // street suffix. Four digits or more stay whole (`409722ST` may be
        // a house number run into the street; see `is_glued_ordinal`).
        if digits_lead <= 3
            && matches!(rest, "ST" | "RD")
            && ordinal_ending(&t[..digits_lead]) != Some(rest)
        {
            return Some(digits_lead);
        }
        return None;
    }
    if alpha_lead > 0 && alpha_lead < n {
        let (letters, rest) = t.split_at(alpha_lead);
        let rest_digits = rest.bytes().all(|c| c.is_ascii_digit());
        // TAMPA33616
        if rest_digits && letters.len() >= 2 && (rest.len() == 5 || rest.len() == 9) {
            return Some(alpha_lead);
        }
        // I10, IH280, CR1185 (a route may end in a letter: 60E).
        let route = rest.bytes().next().is_some_and(|c| c.is_ascii_digit());
        if route && lex.highway(letters).is_some() {
            return Some(alpha_lead);
        }
        // APT107, APT3D
        let unit_value = rest.bytes().next().is_some_and(|c| c.is_ascii_digit())
            && rest.bytes().all(|c| c.is_ascii_alphanumeric());
        if unit_value && letters.len() >= 2 && lex.unit(letters).is_some() {
            return Some(alpha_lead);
        }
        return None;
    }
    if alpha_lead == n && next_numeric && n >= 4 && !known_word(t, lex) {
        // CTAPT 107, DRUNIT 142: a suffix followed by a designator.
        for cut in 2..n - 1 {
            let (l, r) = t.split_at(cut);
            if r.len() >= 2 && lex.suffix(l).is_some() && lex.unit(r).is_some() {
                return Some(cut);
            }
        }
    }
    None
}

/// The ordinal ending English writes after the number `n`: `ST`, `ND`,
/// `RD` or `TH` (11 to 13 take `TH`).
fn ordinal_ending(n: &str) -> Option<&'static str> {
    let b = n.as_bytes();
    let last = *b.last()?;
    let tens = b.len() >= 2 && b[b.len() - 2] == b'1';
    Some(match (tens, last) {
        (false, b'1') => "ST",
        (false, b'2') => "ND",
        (false, b'3') => "RD",
        _ => "TH",
    })
}

/// `STREETDADE` before `CITY FL`: where a street suffix ends and a city the
/// state packs know begins. The city is the rest of the word plus some of
/// the words after it, and must come before a state, a ZIP or another
/// segment (with none of these the tagger could not use the split). Never
/// splits a word that starts a place itself, or ends one with the words
/// before it (`PICO RIVERA` is not `RIV` `ERA`).
///
/// With the national place list almost any word ends in a place name, so the
/// anchor has to be real:
/// - a state or ZIP in the same segment ends the city only when nothing but
///   a ZIP follows it (`DE LA REINA` is not Delaware, `CT` in
///   `CREEKWOOD CT, MIAMI` is not Connecticut), and only when no later
///   segment holds the state or ZIP;
/// - the end of the segment ends it only when the next segment starts with a
///   state or a ZIP (`STRAND, MIAMI, FL` is not `ST` `RAND`), and the city's
///   last word is not a street suffix.
fn glue_place(
    t: &str,
    before: Vec<&str>,
    rest: &[&str],
    later_seg: bool,
    later_anchor: bool,
    lex: &Lexicons,
) -> Option<usize> {
    let n = t.len();
    if n < 5 || !t.bytes().all(|c| c.is_ascii_alphabetic()) || known_word(t, lex) {
        return None;
    }
    // This word ends a place that began earlier in the segment.
    for from in 0..before.len() {
        let mut name = before[from..].join(" ");
        name.push(' ');
        name.push_str(t);
        if lex.is_place(&name) {
            return None;
        }
    }
    let after = &rest[..rest.len().min(MAX_PLACE_WORDS)];
    // The city starting with `first`, ending before word `j` of `after`.
    let city_at = |first: &str| {
        (0..=after.len().min(MAX_PLACE_WORDS - 1)).any(|j| {
            let ends = match after.get(j) {
                // A segment-end anchor takes no street suffix as the city's
                // last word (`DALEWOOD WAY,` is not `DALE` `WOOD WAY`).
                None => {
                    later_seg
                        && j == rest.len()
                        && !after[..j].last().is_some_and(|w| lex.suffix(w).is_some())
                }
                Some(w) => {
                    (lex.state(w).is_some() || zip_parts(w).is_some())
                        && !later_anchor
                        && (j + 1 == rest.len()
                            || (j + 2 == rest.len() && zip_parts(rest[j + 1]).is_some()))
                }
            };
            let mut city = first.to_string();
            for w in &after[..j] {
                city.push(' ');
                city.push_str(w);
            }
            ends && lex.is_place(&city)
        })
    };
    if city_at(t) {
        return None;
    }
    (2..n - 2).find(|&cut| lex.suffix(&t[..cut]).is_some() && city_at(&t[cut..]))
}

fn known_word(t: &str, lex: &Lexicons) -> bool {
    lex.suffix(t).is_some()
        || lex.unit(t).is_some()
        || lex.directional(t).is_some()
        || lex.state(t).is_some()
        || lex.special(t).is_some()
        || lex.highway(t).is_some()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::clean::clean;

    fn toks(s: &str) -> Vec<String> {
        tokenize(&clean(s), None, 0, &Lexicons::builtin())
            .into_iter()
            .map(|t| t.text)
            .collect()
    }

    #[test]
    fn punctuation() {
        assert_eq!(
            toks("4158 Inverrary Dr #309"),
            ["4158", "INVERRARY", "DR", "#", "309"]
        );
        assert_eq!(toks("112 PAUL ST (110)"), ["112", "PAUL", "ST", "110"]);
        assert_eq!(
            toks("3151 COOPER ST -UNIT 15"),
            ["3151", "COOPER", "ST", "UNIT", "15"]
        );
        assert_eq!(toks("N/A - MAGNOLIA AVE"), ["N/A", "-", "MAGNOLIA", "AVE"]);
        assert_eq!(toks("CHATHAM DRIVE:"), ["CHATHAM", "DRIVE"]);
        assert_eq!(
            toks("St. Petersburg, FL- 33701"),
            ["ST", "PETERSBURG", "FL", "33701"]
        );
        assert_eq!(toks("P.O. Box 7"), ["PO", "BOX", "7"]);
        assert_eq!(toks("ST.CLOUD FT.MYERS"), ["ST", "CLOUD", "FT", "MYERS"]);
        assert_eq!(toks("U.S.A. N.W. P.O.BOX"), ["USA", "NW", "POBOX"]);
        assert_eq!(toks("HARBOR BLV.D CR.N"), ["HARBOR", "BLVD", "CR", "N"]);
        assert_eq!(toks("PO.BOX.2351"), ["PO", "BOX", "2351"]);
        assert_eq!(toks("39.2 RD"), ["39.2", "RD"]);
        assert_eq!(toks("C-22 1204-1206"), ["C-22", "1204-1206"]);
    }

    #[test]
    fn glued() {
        assert_eq!(toks("101BRADDOCK LN"), ["101", "BRADDOCK", "LN"]);
        assert_eq!(toks("42ND ST 12A"), ["42ND", "ST", "12A"]);
        assert_eq!(toks("TAMPA33616"), ["TAMPA", "33616"]);
        assert_eq!(toks("I10 IH280"), ["I", "10", "IH", "280"]);
        assert_eq!(
            toks("BIRD BAY CTAPT 107"),
            ["BIRD", "BAY", "CT", "APT", "107"]
        );
        assert_eq!(toks("ORLANDO DRUNIT 142"), ["ORLANDO", "DR", "UNIT", "142"]);
        assert_eq!(toks("APT107"), ["APT", "107"]);
        assert_eq!(toks("ACCESS UNDETERMINED"), ["ACCESS", "UNDETERMINED"]);
        assert_eq!(toks("409742ND SQ"), ["409742ND", "SQ"]);
        assert_eq!(toks("NE 178ST"), ["NE", "178", "ST"]);
        assert_eq!(toks("NW 54RD 112ST"), ["NW", "54", "RD", "112", "ST"]);
        assert_eq!(
            toks("21ST 3RD 11ST 101ST"),
            ["21ST", "3RD", "11", "ST", "101ST"]
        );
        assert_eq!(toks("409722ST"), ["409722ST"]);
    }

    #[test]
    fn designator_joined_to_its_value() {
        assert_eq!(toks("APT.1285"), ["APT", "1285"]);
        assert_eq!(toks("STE.200 APT:201"), ["STE", "200", "APT", "201"]);
        assert_eq!(toks("UNIT-4 APT.D"), ["UNIT", "4", "APT", "D"]);
        assert_eq!(toks("APT3D"), ["APT", "3D"]);
        // Not a designator: a route, a name with a period, a hyphenated value.
        assert_eq!(toks("GA-25"), ["GA", "25"]);
        assert_eq!(toks("ST.LUCIE"), ["ST", "LUCIE"]);
        assert_eq!(toks("C-22"), ["C-22"]);
    }

    #[cfg(feature = "state-fl")]
    #[test]
    fn glued_city() {
        assert_eq!(
            toks("13551 WILSON STREETDADE CITY FL 33525"),
            ["13551", "WILSON", "STREET", "DADE", "CITY", "FL", "33525"]
        );
        assert_eq!(
            toks("1 OAK AVETITUSVILLE, FL"),
            ["1", "OAK", "AVE", "TITUSVILLE", "FL"]
        );
        // Nothing after the city to anchor it, or no suffix before it.
        assert_eq!(
            toks("13551 WILSON STREETDADE CITY"),
            ["13551", "WILSON", "STREETDADE", "CITY"]
        );
        assert_eq!(toks("1 PARKLAND DR"), ["1", "PARKLAND", "DR"]);
    }

    #[test]
    fn segments_and_spans() {
        let input = "26166 Paysandu Dr, PUNTA GORDA, FL- 33983";
        let t = tokenize(&clean(input), None, 0, &Lexicons::builtin());
        let segs: Vec<_> = t.iter().map(|t| t.seg).collect();
        assert_eq!(segs, [0, 0, 0, 1, 1, 2, 2]);
        let fl = &t[5];
        assert_eq!(&input[fl.span.start..fl.span.end], "FL-");
        let p = tokenize(&clean("(OFF)"), None, 0, &Lexicons::builtin());
        assert_eq!(p[0].text, "OFF");
        assert!(p[0].paren);
        assert_eq!((p[0].span.start, p[0].span.end), (0, 5));
    }
}
