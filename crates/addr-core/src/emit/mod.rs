//! Emitters: read-only consumers of [`ParsedAddress`](crate::ParsedAddress).
//! They never mutate or re-parse it.

mod key;
mod reso;
mod usps;

pub use key::KeyOptions;
pub use reso::ResoAddress;
pub use usps::UspsAddress;
