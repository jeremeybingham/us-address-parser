//! # addr-core
//!
//! Pure-Rust US address parsing and normalization.
//!
//! This crate **parses and formats** addresses. It does **not** validate
//! deliverability; that requires CASS-certified data.
//!
//! Status: the public types and the lexicon tables ([`Lexicons`]) are in
//! place; the parsing pipeline is not implemented yet. See `docs/design.md`
//! for the milestone plan.

mod input;
pub mod lexicon;
mod parser;
mod types;

pub use input::Fields;
pub use lexicon::{Lexicons, Lookup, MatchKind};
pub use parser::{ParseError, Parser};
pub use types::{
    Diagnostics, Directional, InputKind, ParsedAddress, RouteKind, RuralRoute, SourceInfo,
    StateCode, StateKind, StreetSuffix, UnitType, Warning,
};

/// Library version, surfaced in [`Diagnostics`].
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
