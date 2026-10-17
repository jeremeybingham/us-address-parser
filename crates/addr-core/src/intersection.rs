//! Intersections (item 55, decision 37).
//!
//! `MAIN ST & 1ST AVE` names a corner, not a place with a house number. The
//! parser reads the first street as usual, leaves the second one (the
//! connector and the street after it) in `unparsed`, raises
//! [`crate::Warning::IntersectionDetected`] and gives no key, since the first
//! street alone would match every address on it. There are no cross-street
//! fields.
//!
//! The second street is cut from the tokens before tagging so it cannot be
//! read as part of the first or as a city, then put back as unparsed.
//!
//! Only a shape that cannot be one street name counts:
//! - the connector is `&`, `@`, `AND` or `AT`, in one comma segment with
//!   both streets;
//! - the words before it (after a house number or directional) are a name
//!   and a street suffix, optionally a directional: `MAIN ST`, `N 8TH ST NW`;
//!   a suffix word alone is a name (`BEACH AT SEA DR`);
//! - the words after it run to a street suffix. A suffix directly before
//!   another is part of the name (`PARK AVE`).
//!
//! `LAND AND SEA RD`, `STATE AND MAIN ST` and `AT&T WAY` are single streets.

use std::ops::Range;

use crate::lexicon::Lexicons;
use crate::tagger::Tag;
use crate::tokenize::Token;

const CONNECTORS: [&str; 4] = ["&", "@", "AND", "AT"];

/// The tokens of the second street, from its connector through its suffix
/// (and a directional after it), if the input is an intersection.
pub(crate) fn find(toks: &[Token], lex: &Lexicons) -> Option<Range<usize>> {
    let is_suffix = |t: &Token| lex.suffix(&t.text).is_some();
    let is_dir = |t: &Token| lex.directional(&t.text).is_some();
    for (c, conn) in toks.iter().enumerate() {
        if !CONNECTORS.contains(&conn.text.as_str()) {
            continue;
        }
        // The segment's tokens before and after the connector.
        let before = toks[..c]
            .iter()
            .rev()
            .take_while(|t| t.seg == conn.seg)
            .count();
        let left = &toks[c - before..c];
        let right_len = toks[c + 1..]
            .iter()
            .take_while(|t| t.seg == conn.seg)
            .count();
        let right = &toks[c + 1..c + 1 + right_len];

        // The first street: a name, then a suffix, then maybe a directional.
        let mut words = left;
        while let Some((t, rest)) = words.split_first() {
            if t.text.bytes().all(|b| b.is_ascii_digit()) || is_dir(t) {
                words = rest;
            } else {
                break;
            }
        }
        if let [.., last] = words {
            if words.len() >= 2 && is_dir(last) && is_suffix(&words[words.len() - 2]) {
                words = &words[..words.len() - 1];
            }
        }
        if words.len() < 2 || !words.last().is_some_and(is_suffix) {
            continue;
        }

        // The second street: up to a suffix that no suffix follows.
        let end = (0..right.len())
            .find(|&k| is_suffix(&right[k]) && !right.get(k + 1).is_some_and(is_suffix))?;
        let mut stop = c + 1 + end + 1;
        if toks
            .get(stop)
            .is_some_and(|t| t.seg == conn.seg && is_dir(t))
        {
            stop += 1;
        }
        return Some(c..stop);
    }
    None
}

/// The tokens without `cut`.
pub(crate) fn remove(toks: &[Token], cut: &Range<usize>) -> Vec<Token> {
    toks[..cut.start]
        .iter()
        .chain(&toks[cut.end..])
        .cloned()
        .collect()
}

/// A token index in the full list, as an index after `cut` was removed.
pub(crate) fn shift(i: usize, cut: &Range<usize>) -> usize {
    if i <= cut.start {
        i
    } else {
        i - (i.min(cut.end) - cut.start)
    }
}

/// Tags for the full list: those of the shortened list, with the cut tokens
/// unparsed.
pub(crate) fn restore(tags: Vec<Tag>, cut: &Range<usize>) -> Vec<Tag> {
    let mut out = tags;
    let at = cut.start.min(out.len());
    out.splice(at..at, std::iter::repeat(Tag::Unparsed).take(cut.len()));
    out
}
