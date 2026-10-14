//! Tokenizer (pipeline step 2, item 11).
//!
//! Splits cleaned text into tokens that keep their byte span in the
//! original input. Commas and semicolons separate *segments* (address lines
//! or parts of one), which the tagger uses as strong boundaries.
//!
//! Punctuation rules, from the shapes in `docs/prior-art.md` and Pub 28:
//! - `#` is always its own token: `#309` -> `#` `309`.
//! - Parentheses are dropped from the text but kept in the span, and the
//!   token is marked `paren`: `(110)` -> `110`.
//! - A hyphen alone is a separator token (`N/A - MAGNOLIA AVE`). Leading and
//!   trailing hyphens are dropped (`-UNIT`, `FL-`); inner ones are kept
//!   (`C-22`, `1204-1206`).
//! - Periods are dropped except between digits (`ST.` -> `ST`, `39.2`).
//! - Trailing colons and quotes are dropped.
//!
//! Glued tokens are split when the pieces are unambiguous:
//! - house number run into a name: `101BRADDOCK` -> `101` `BRADDOCK`, but
//!   not an ordinal (`42ND`) or a one-letter suffix (`12A`);
//! - a name run into a ZIP: `TAMPA33616` -> `TAMPA` `33616`;
//! - a highway prefix run into its route: `I10`, `IH280` (Pub 28 Appendix F);
//! - a unit designator run into its number: `APT107`;
//! - a street suffix run into a unit designator, before a number:
//!   `CTAPT 107` -> `CT` `APT` `107`.

use crate::classify::{is_ordinal, ORDINAL_SUFFIXES};
use crate::clean::Cleaned;
use crate::lexicon::Lexicons;
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
        let mut p = Piece {
            cleaned,
            field,
            seg,
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
    out: &'a mut Vec<Token>,
}

impl Piece<'_> {
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
        // Trim wrapping punctuation from the text, but keep the whole piece
        // in the span when it is a single token.
        let trim: &[char] = &['(', ')', '-', ':', '"', '\'', '.', '[', ']'];
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

        if t.bytes().all(|c| c.is_ascii_alphanumeric()) {
            if let Some(cut) = glue_split(t, next_numeric, lex) {
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

/// Where to split a glued alphanumeric token, if anywhere.
fn glue_split(t: &str, next_numeric: bool, lex: &Lexicons) -> Option<usize> {
    let digits_lead = t.bytes().take_while(u8::is_ascii_digit).count();
    let alpha_lead = t.bytes().take_while(u8::is_ascii_alphabetic).count();
    let n = t.len();
    if digits_lead > 0 && digits_lead < n {
        let rest = &t[digits_lead..];
        // 101BRADDOCK; not 12A, 42ND, 1ST.
        if rest.len() >= 2
            && rest.bytes().all(|c| c.is_ascii_alphabetic())
            && !ORDINAL_SUFFIXES.contains(&rest)
            && !is_ordinal(t)
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
        // APT107
        if rest_digits && letters.len() >= 2 && lex.unit(letters).is_some() {
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
