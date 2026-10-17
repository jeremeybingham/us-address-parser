//! Trailing country (item 54, decision 36).
//!
//! `MIAMI, FL 33101, USA` and `... FL 33101 UNITED STATES` carry a country
//! the parser does not model. The tail is cut from the token list before
//! tagging, so it reaches neither the city nor `unparsed`, and the caller
//! raises [`crate::Warning::CountryIgnored`]. Only names of the United States
//! count (`data/special.toml` `[country]`); another country stays in the
//! parse as ordinary text.
//!
//! A name is cut only where it cannot be part of the address:
//! - it is a segment of its own after a comma (`MIAMI, USA`), or a whole
//!   field other than the street line; or
//! - it follows a ZIP or a state (`FL 33101 USA`, `FL US`).
//!
//! So `US` in `123 US HWY 1`, or after a street (`123 MAIN ST US`), stays.

use crate::classify::zip_parts;
use crate::lexicon::Lexicons;
use crate::tokenize::Token;

/// The longest country name, in words (`UNITED STATES OF AMERICA`).
const MAX_WORDS: usize = 4;

/// Cut a trailing country from `toks`; true when one was cut. With
/// `own_field`, the tokens are a whole field that is not the street line,
/// and a country that is all of it is cut too; otherwise something must
/// precede the country.
pub(crate) fn strip_trailing(toks: &mut Vec<Token>, own_field: bool, lex: &Lexicons) -> bool {
    let n = toks.len();
    for words in (1..=MAX_WORDS.min(n)).rev() {
        let k = n - words;
        let tail = &toks[k..];
        if tail.iter().any(|t| t.seg != tail[0].seg) {
            continue;
        }
        let text = tail.iter().map(|t| t.text.as_str()).collect::<Vec<_>>();
        if !lex.is_country(&text.join(" ")) {
            continue;
        }
        let cut = if k == 0 {
            own_field
        } else {
            let prev = &toks[k - 1];
            prev.seg != toks[k].seg
                || zip_parts(&prev.text).is_some()
                || ends_with_state(&toks[..k], lex)
        };
        if cut {
            toks.truncate(k);
            return true;
        }
    }
    false
}

/// Whether the tokens end in a state: one word (`FL`) or two (`NEW YORK`).
fn ends_with_state(toks: &[Token], lex: &Lexicons) -> bool {
    let n = toks.len();
    let last = &toks[n - 1].text;
    lex.state(last).is_some()
        || (n >= 2
            && toks[n - 2].seg == toks[n - 1].seg
            && lex.state(&format!("{} {last}", toks[n - 2].text)).is_some())
}
