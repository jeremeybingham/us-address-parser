//! # addr-core
//!
//! Pure-Rust US address parsing and normalization.
//!
//! This crate **parses and formats** addresses. It does **not** validate
//! deliverability; that requires CASS-certified data.
//!
//! Status: skeleton. The parsing pipeline is not implemented yet; see
//! `docs/design.md` for the milestone plan.

mod input;
mod parser;
mod types;

pub use input::Fields;
pub use parser::{ParseError, Parser};
pub use types::{
    Diagnostics, Directional, InputKind, ParsedAddress, RuralRoute, SourceInfo, StateCode,
    UnitType, Warning,
};

/// Library version, surfaced in [`Diagnostics`].
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
