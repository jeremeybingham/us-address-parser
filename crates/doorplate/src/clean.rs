//! Input cleaning (pipeline step 1).
//!
//! Produces uppercase ASCII-leaning text with single spaces, and keeps a map
//! from every cleaned byte back to the original input so components can
//! report spans into the original text.
//!
//! What it does:
//! - Any Unicode whitespace or control character, including the
//!   non-breaking space seen in real license files (`BOYNTON\xa0BEACH`),
//!   becomes a space; runs of spaces collapse to one; ends are trimmed.
//! - Accented Latin letters lose their accent (`RATÓN` -> `RATON`).
//! - Curly quotes and Unicode dashes become their ASCII forms.
//! - Letters are uppercased.
//!
//! Punctuation is left for the tokenizer, which knows where commas, `#`,
//! hyphens and periods matter.

/// Cleaned text plus the map back to the original.
#[derive(Debug, Clone)]
pub(crate) struct Cleaned {
    pub text: String,
    /// For each byte of `text`, the original byte range of the character it
    /// came from.
    src: Vec<(usize, usize)>,
}

impl Cleaned {
    /// Original byte range covered by `text[start..end]`. `start < end`
    /// must hold; an empty or out-of-range request returns `None`.
    pub fn original_range(&self, start: usize, end: usize) -> Option<(usize, usize)> {
        if start >= end {
            return None;
        }
        let (s, _) = *self.src.get(start)?;
        let (_, e) = *self.src.get(end - 1)?;
        Some((s, e))
    }
}

pub(crate) fn clean(input: &str) -> Cleaned {
    let mut text = String::with_capacity(input.len());
    let mut src = Vec::with_capacity(input.len());
    let mut pending_space = false;
    for (i, c) in input.char_indices() {
        let range = (i, i + c.len_utf8());
        if c.is_whitespace() || c.is_control() {
            pending_space = !text.is_empty();
            continue;
        }
        if pending_space {
            // A space between two kept characters maps to the gap's start.
            text.push(' ');
            src.push((range.0, range.0));
            pending_space = false;
        }
        let mut push = |s: &str| {
            src.extend(std::iter::repeat(range).take(s.len()));
            text.push_str(s);
        };
        match fold(c) {
            Some(s) => push(s),
            None => {
                for u in c.to_uppercase() {
                    let mut buf = [0u8; 4];
                    push(u.encode_utf8(&mut buf));
                }
            }
        }
    }
    Cleaned { text, src }
}

/// ASCII replacement for a non-ASCII character, uppercased; `None` keeps
/// the character (uppercased by the caller).
fn fold(c: char) -> Option<&'static str> {
    if c.is_ascii() {
        return None;
    }
    Some(match c {
        'À'..='Å' | 'à'..='å' | 'Ā' | 'ā' | 'Ă' | 'ă' | 'Ą' | 'ą' => "A",
        'Æ' | 'æ' => "AE",
        'Ç' | 'ç' | 'Ć' | 'ć' | 'Č' | 'č' => "C",
        'Ð' | 'ð' | 'Ď' | 'ď' | 'Đ' | 'đ' => "D",
        'È'..='Ë' | 'è'..='ë' | 'Ē' | 'ē' | 'Ė' | 'ė' | 'Ę' | 'ę' | 'Ě' | 'ě' => "E",
        'Ğ' | 'ğ' => "G",
        'Ì'..='Ï' | 'ì'..='ï' | 'Ī' | 'ī' | 'Į' | 'į' | 'İ' | 'ı' => "I",
        'Ł' | 'ł' => "L",
        'Ñ' | 'ñ' | 'Ń' | 'ń' | 'Ň' | 'ň' => "N",
        'Ò'..='Ö' | 'Ø' | 'ò'..='ö' | 'ø' | 'Ō' | 'ō' | 'Ő' | 'ő' => "O",
        'Œ' | 'œ' => "OE",
        'Ř' | 'ř' => "R",
        'Ś' | 'ś' | 'Š' | 'š' | 'Ş' | 'ş' => "S",
        'ß' => "SS",
        'Ť' | 'ť' | 'Ţ' | 'ţ' => "T",
        'Ù'..='Ü' | 'ù'..='ü' | 'Ū' | 'ū' | 'Ů' | 'ů' | 'Ű' | 'ű' => "U",
        'Ý' | 'ý' | 'ÿ' | 'Ÿ' => "Y",
        'Ź' | 'ź' | 'Ż' | 'ż' | 'Ž' | 'ž' => "Z",
        '‘' | '’' | '‚' | '′' => "'",
        '“' | '”' | '„' | '″' => "\"",
        '‐' | '‑' | '‒' | '–' | '—' | '―' | '−' => "-",
        '＃' | '№' => "#",
        '½' => "1/2",
        '¼' => "1/4",
        '¾' => "3/4",
        _ => return None,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn folds_and_collapses() {
        let c = clean("  Boca\u{a0}Ratón ,  fl ");
        assert_eq!(c.text, "BOCA RATON , FL");
    }

    #[test]
    fn maps_back_to_original() {
        let input = "1  Ñame\u{a0}St";
        let c = clean(input);
        assert_eq!(c.text, "1 NAME ST");
        let (s, e) = c.original_range(2, 6).unwrap();
        assert_eq!(&input[s..e], "Ñame");
        let (s, e) = c.original_range(7, 9).unwrap();
        assert_eq!(&input[s..e], "St");
    }

    #[test]
    fn dashes_and_quotes() {
        assert_eq!(clean("N/A – Main’s").text, "N/A - MAIN'S");
    }
}
