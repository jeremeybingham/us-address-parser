//! Pre-split input: the data is cut at U+0001 into line 1, line 2, city,
//! state and ZIP. Nothing may panic, and the emitters must run.
#![no_main]

use doorplate::{Fields, KeyOptions, Parser};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &str| {
    let mut parts = text.split('\u{1}').map(|s| Some(s.to_string()));
    let fields = Fields {
        line1: parts.next().flatten(),
        line2: parts.next().flatten(),
        city: parts.next().flatten(),
        state: parts.next().flatten(),
        zip: parts.next().flatten(),
    };
    if let Ok(a) = Parser::new().parse_fields(&fields) {
        let _ = a.key(KeyOptions::property());
        let _ = a.key(KeyOptions::unit());
        let _ = a.slug();
        let _ = a.to_usps();
        let _ = a.to_reso();
        let _ = a.tag();
    }
});
