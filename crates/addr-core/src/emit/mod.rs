//! Emitters: read-only consumers of [`ParsedAddress`](crate::ParsedAddress).
//! They never mutate or re-parse it.

mod key;
mod reso;
mod slug;
mod usaddress;
mod usps;

pub use key::KeyOptions;
pub use reso::ResoAddress;
pub use usaddress::UsaddressTag;
pub use usps::UspsAddress;
