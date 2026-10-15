//! Token classification (pipeline step 3, item 12).
//!
//! Shape tests on single tokens (numbers, ZIPs, ordinals, unit values) and
//! longest-match lexicon lookups over runs of tokens. The tagger decides
//! what a token *is* from these and its position; nothing here looks at
//! context beyond the tokens it is given.

use crate::lexicon::{Lexicons, MAX_FORM_WORDS};
use crate::tokenize::Token;

pub(crate) const ORDINAL_SUFFIXES: [&str; 4] = ["ST", "ND", "RD", "TH"];

pub(crate) fn has_digit(t: &str) -> bool {
    t.bytes().any(|c| c.is_ascii_digit())
}

pub(crate) fn all_digits(t: &str) -> bool {
    !t.is_empty() && t.bytes().all(|c| c.is_ascii_digit())
}

/// `42ND`, `1ST`, `6TH`: digits then an ordinal suffix.
pub(crate) fn is_ordinal(t: &str) -> bool {
    let d = t.bytes().take_while(u8::is_ascii_digit).count();
    d > 0 && ORDINAL_SUFFIXES.contains(&&t[d..])
}

/// A number run into an ordinal street that can't be split reliably:
/// `409742ND` is 4097 42ND or 40974 2ND. Ordinal street numbers rarely
/// run past three digits.
pub(crate) fn is_glued_ordinal(t: &str) -> bool {
    is_ordinal(t) && t.bytes().take_while(u8::is_ascii_digit).count() >= 4
}

/// `5` or `9` digits, or `5-4`. Returns (zip5, zip4).
pub(crate) fn zip_parts(t: &str) -> Option<(&str, Option<&str>)> {
    let b = t.as_bytes();
    if all_digits(t) {
        return match b.len() {
            5 => Some((t, None)),
            9 => Some((&t[..5], Some(&t[5..]))),
            _ => None,
        };
    }
    let (a, z4) = t.split_once('-')?;
    (a.len() == 5 && all_digits(a) && z4.len() == 4 && all_digits(z4)).then_some((a, Some(z4)))
}

/// Shapes a house number takes: `123`, `12A`, `12-A`, `1204-1206`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NumberShape {
    Plain,
    /// Two numbers joined by a hyphen: maybe a range of addresses.
    Range,
}

pub(crate) fn house_number(t: &str) -> Option<NumberShape> {
    let d = t.bytes().take_while(u8::is_ascii_digit).count();
    if d == 0 {
        return None;
    }
    let rest = &t[d..];
    if rest.is_empty() {
        return Some(NumberShape::Plain);
    }
    if rest.len() == 1 && rest.bytes().all(|c| c.is_ascii_alphabetic()) {
        return Some(NumberShape::Plain);
    }
    let after = rest.strip_prefix('-')?;
    if all_digits(after) {
        // `1204-1206` is a range; `112-10` and `289-01` are one hyphenated
        // address (Pub 28 Appendix D1, section 232).
        let range = after.len() == d && after > &t[..d];
        return Some(if range {
            NumberShape::Range
        } else {
            NumberShape::Plain
        });
    }
    (after.len() == 1 && after.bytes().all(|c| c.is_ascii_alphabetic()))
        .then_some(NumberShape::Plain)
}

/// `1/2`, `3/4`: the fraction in `123 1/2`.
pub(crate) fn is_fraction(t: &str) -> bool {
    t.split_once('/')
        .is_some_and(|(a, b)| all_digits(a) && all_digits(b) && a.len() <= 2 && b.len() <= 2)
}

/// Two numbers joined by a slash: `5210/5212` (several addresses).
pub(crate) fn is_slash_pair(t: &str) -> bool {
    t.split_once('/')
        .is_some_and(|(a, b)| house_number(a).is_some() && house_number(b).is_some())
        && !is_fraction(t)
}

/// A unit or box value: has a digit (`309`, `4C`, `H28`, `A4-2`, `3-203`)
/// or is one letter (`B`). Letters, digits and inner hyphens only.
pub(crate) fn is_unit_value(t: &str) -> bool {
    let ok_chars = t.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-');
    let single_letter = t.len() == 1 && t.bytes().all(|c| c.is_ascii_alphabetic());
    ok_chars && !t.starts_with('-') && (has_digit(t) || single_letter)
}

/// A highway route: starts with a digit (`20`, `60E`, `1150EE`).
pub(crate) fn is_route_value(t: &str) -> bool {
    t.bytes().next().is_some_and(|c| c.is_ascii_digit())
        && t.bytes().all(|c| c.is_ascii_alphanumeric())
}

/// A single letter, as in `V 21` or `AVENUE G`.
pub(crate) fn is_letter(t: &str) -> bool {
    t.len() == 1 && t.bytes().all(|c| c.is_ascii_alphabetic())
}

/// Longest run of tokens starting at `start` (and before `end`, within one
/// segment) that `f` accepts, joined with spaces. Returns the value and the
/// number of tokens.
pub(crate) fn longest_at<T>(
    toks: &[Token],
    start: usize,
    end: usize,
    f: impl Fn(&str) -> Option<T>,
) -> Option<(T, usize)> {
    let seg = toks.get(start)?.seg;
    let max = MAX_FORM_WORDS.min(end.saturating_sub(start));
    (1..=max).rev().find_map(|w| {
        let run = toks.get(start..start + w)?;
        if run
            .iter()
            .any(|t| t.seg != seg || t.is_hash() || t.is_dash())
        {
            return None;
        }
        f(&join(run)).map(|v| (v, w))
    })
}

/// Longest run of tokens ending just before `end` (and starting at or after
/// `min`, within one segment) that `f` accepts.
pub(crate) fn longest_ending<T>(
    toks: &[Token],
    min: usize,
    end: usize,
    f: impl Fn(&str) -> Option<T>,
) -> Option<(T, usize)> {
    let seg = toks.get(end.checked_sub(1)?)?.seg;
    let max = MAX_FORM_WORDS.min(end.saturating_sub(min));
    (1..=max).rev().find_map(|w| {
        let run = toks.get(end - w..end)?;
        if run
            .iter()
            .any(|t| t.seg != seg || t.is_hash() || t.is_dash())
        {
            return None;
        }
        f(&join(run)).map(|v| (v, w))
    })
}

pub(crate) fn join(run: &[Token]) -> String {
    let mut s = String::new();
    for t in run {
        if !s.is_empty() {
            s.push(' ');
        }
        s.push_str(&t.text);
    }
    s
}

/// Whether `t` is a cut-off unit designator: a strict prefix, two letters
/// or more, of a designator's abbreviation or name (`UNI`, `APAR`). A street
/// suffix is not: `ST` and `TRL` start STE and TRAILER, but `DRY CREEK TRL`
/// ends in its suffix.
pub(crate) fn is_cut_designator(t: &str, lex: &Lexicons) -> bool {
    t.len() >= 2
        && t.bytes().all(|c| c.is_ascii_alphabetic())
        && lex.unit(t).is_none()
        && lex.suffix(t).is_none()
        && lex.units().any(|u| {
            [u.abbr(), u.name()]
                .iter()
                .any(|w| w.len() > t.len() && w.starts_with(t))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes() {
        assert_eq!(zip_parts("33101"), Some(("33101", None)));
        assert_eq!(zip_parts("33101-1234"), Some(("33101", Some("1234"))));
        assert_eq!(zip_parts("331011234"), Some(("33101", Some("1234"))));
        assert_eq!(zip_parts("3310"), None);
        assert_eq!(house_number("12A"), Some(NumberShape::Plain));
        assert_eq!(house_number("12-A"), Some(NumberShape::Plain));
        assert_eq!(house_number("1204-1206"), Some(NumberShape::Range));
        let lex = Lexicons::builtin();
        assert!(is_cut_designator("UNI", &lex));
        assert!(!is_cut_designator("TRL", &lex) && !is_cut_designator("ST", &lex));
        assert_eq!(house_number("112-10"), Some(NumberShape::Plain));
        assert_eq!(house_number("289-01"), Some(NumberShape::Plain));
        assert_eq!(house_number("42ND"), None);
        assert!(is_ordinal("42ND") && !is_glued_ordinal("42ND"));
        assert!(is_glued_ordinal("409742ND"));
        assert!(is_slash_pair("5210/5212") && !is_slash_pair("1/2"));
        assert!(is_fraction("1/2"));
        for v in ["309", "4C", "H28", "A4-2", "3-203", "B"] {
            assert!(is_unit_value(v), "{v}");
        }
        assert!(!is_unit_value("OFF"));
        assert!(is_route_value("60E") && !is_route_value("MM"));
    }

    #[test]
    fn cut_designator() {
        let lex = Lexicons::builtin();
        assert!(is_cut_designator("UNI", &lex));
        assert!(!is_cut_designator("UNIT", &lex));
        assert!(!is_cut_designator("CUTOF", &lex));
    }
}
