//! Emitters: read-only consumers of [`ParsedAddress`](crate::ParsedAddress).
//! They never mutate or re-parse it.

mod key;
mod usps;

pub use key::KeyOptions;
pub use usps::UspsAddress;
