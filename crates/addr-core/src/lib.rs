//! # addr-core
//!
//! Pure-Rust US address parsing and normalization.
//!
//! This crate **parses and formats** addresses. It does **not** validate
//! deliverability; that requires CASS-certified data.
//!
//! ```
//! let a = addr_core::Parser::new()
//!     .parse("123 N Main St Apt 4, Miami, FL 33101")
//!     .unwrap();
//! assert_eq!(a.street_name.as_deref(), Some("MAIN"));
//! assert_eq!(a.street_suffix.map(|s| s.abbr()), Some("ST"));
//! assert_eq!(a.city.as_deref(), Some("MIAMI"));
//! ```
//!
//! Status: parsing (Milestone 2) and the canonical key are in place; the USPS,
//! RESO and slug emitters come next (Milestone 3). See `docs/design.md` for the plan.

mod assemble;
mod classify;
mod clean;
mod emit;
mod input;
pub mod lexicon;
mod parser;
mod tagger;
mod tokenize;
mod types;

pub use emit::KeyOptions;
pub use input::Fields;
pub use lexicon::{HighwayPrefix, Lexicons, Lookup, MatchKind};
pub use parser::{ParseError, Parser};
pub use types::{
    Component, ComponentSpan, Diagnostics, Directional, FieldName, Highway, InputKind, MilitaryBox,
    MilitaryKind, MilitaryPostOffice, ParsedAddress, RouteKind, RuralRoute, SourceInfo, Span,
    StateCode, StateKind, StreetSuffix, UnitType, Warning,
};

/// Library version, surfaced in [`Diagnostics`].
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
