//! Any text must parse (or be rejected as empty) without panicking, and
//! every emitter must run on the result. The first byte picks a
//! `default_state` so the hinted paths are covered too.
#![no_main]

use doorplate::{KeyOptions, ParseOptions, Parser};
use libfuzzer_sys::fuzz_target;

const HINTS: [Option<&str>; 4] = [None, Some("FL"), Some("AL"), Some("NY")];

fuzz_target!(|data: &[u8]| {
    let Some((&pick, rest)) = data.split_first() else {
        return;
    };
    let Ok(text) = std::str::from_utf8(rest) else {
        return;
    };
    let options = ParseOptions {
        default_state: HINTS[usize::from(pick) % HINTS.len()].map(str::to_string),
    };
    let Ok(parser) = Parser::with_options(options) else {
        return;
    };
    if let Ok(a) = parser.parse(text) {
        emit_all(&a);
    }
});

fn emit_all(a: &doorplate::ParsedAddress) {
    let _ = a.key(KeyOptions::property());
    let _ = a.key(KeyOptions::unit());
    let _ = a.slug();
    let _ = a.to_usps();
    let _ = a.to_usps_wrapped(30);
    let _ = a.to_reso();
    let _ = a.tag();
    let _ = a.address_type();
    let _ = a.to_usaddress_tags();
}
