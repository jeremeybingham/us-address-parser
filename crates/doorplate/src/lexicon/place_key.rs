// Place-name comparison key. Like `key.rs`, this file is compiled twice:
// into the crate as `lexicon::place_key`, and into build.rs via `#[path]`, so
// the compiled place tables and the runtime lookups compare names with
// exactly the same rule. Keep it dependency-free and free of `crate::` paths.

/// The comparison key for a place name. Case, punctuation, spacing and
/// abbreviations do not count, so `St. Petersburg`, `SAINT PETERSBURG` and
/// `ST PETERSBURG` share a key, as do `W PALM BCH` and `WEST PALM BEACH`.
///
/// Each word is upper-cased and stripped of anything but letters and digits
/// (hyphens and slashes split words). `ST`, `FT` and `MT` become `SAINT`,
/// `FORT` and `MOUNT`; any other word is replaced by `expand(word)` when it
/// returns a full form (the caller passes directional and Pub 28 suffix
/// names, so `N` is `NORTH` and `SPGS` is `SPRINGS`). The words are joined
/// with no spaces (`DE FUNIAK` is `DEFUNIAK`).
pub fn place_key<F>(s: &str, expand: F) -> String
where
    F: Fn(&str) -> Option<String>,
{
    let mut out = String::with_capacity(s.len());
    for word in s.split(|c: char| c.is_whitespace() || c == '-' || c == '/') {
        let w: String = word
            .chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_uppercase)
            .collect();
        match w.as_str() {
            "" => {}
            "ST" => out.push_str("SAINT"),
            "FT" => out.push_str("FORT"),
            "MT" => out.push_str("MOUNT"),
            _ => match expand(&w) {
                Some(full) => out.push_str(&full),
                None => out.push_str(&w),
            },
        }
    }
    out
}
