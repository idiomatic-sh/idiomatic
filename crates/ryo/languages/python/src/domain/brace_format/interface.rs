//! Static parser port for brace-format syntax.
use super::types::BraceFormatResult;

/// A deterministic parser selected by the language composition.
pub(crate) trait BraceFormatSyntax {
    /// Parse a decoded Python string-format input.
    fn parse(input: &str) -> BraceFormatResult;
}
