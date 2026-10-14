//! Public parser entry point.

use std::fmt;

use crate::input::Fields;
use crate::types::{InputKind, ParsedAddress, Warning};

/// Returned only for truly unusable input. Partial parses are never errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    EmptyInput,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::EmptyInput => f.write_str("address input is empty"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Address parser. Cheap to construct, `Send + Sync`, no global state.
#[derive(Debug, Clone, Default)]
pub struct Parser {}

impl Parser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse a single-line address.
    ///
    /// Skeleton behavior: returns the input as the unparsed remainder with a
    /// [`Warning::NotImplemented`].
    pub fn parse(&self, input: &str) -> Result<ParsedAddress, ParseError> {
        self.parse_inner(input, InputKind::SingleLine)
    }

    /// Parse pre-split fields.
    pub fn parse_fields(&self, fields: &Fields) -> Result<ParsedAddress, ParseError> {
        self.parse_inner(&fields.joined(), InputKind::Fields)
    }

    /// Parse many single-line addresses. Each element succeeds or fails independently.
    pub fn parse_many(&self, inputs: &[&str]) -> Vec<Result<ParsedAddress, ParseError>> {
        inputs.iter().map(|s| self.parse(s)).collect()
    }

    fn parse_inner(&self, input: &str, kind: InputKind) -> Result<ParsedAddress, ParseError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(ParseError::EmptyInput);
        }
        let mut addr = ParsedAddress::empty(input.to_string(), kind);
        addr.diagnostics.unparsed = Some(trimmed.to_string());
        addr.diagnostics.warn(Warning::NotImplemented);
        Ok(addr)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_is_an_error() {
        assert_eq!(Parser::new().parse("   "), Err(ParseError::EmptyInput));
    }

    #[test]
    fn skeleton_returns_diagnostics_not_panic() {
        let a = Parser::new()
            .parse("123 N Main St, Miami FL 33101")
            .unwrap();
        assert!(a.diagnostics.warnings.contains(&Warning::NotImplemented));
        assert_eq!(a.source.kind, InputKind::SingleLine);
    }

    #[test]
    fn fields_are_joined() {
        let f = Fields {
            line1: Some("1 Main St".into()),
            city: Some("Miami".into()),
            state: Some("FL".into()),
            ..Default::default()
        };
        let a = Parser::new().parse_fields(&f).unwrap();
        assert_eq!(a.source.original, "1 Main St, Miami, FL");
        assert_eq!(a.source.kind, InputKind::Fields);
    }
}
