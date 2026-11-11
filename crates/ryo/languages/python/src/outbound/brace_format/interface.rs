//! Static parser port for brace-format syntax.
use crate::domain::brace_format::types::BraceFormatResult;

/// A deterministic parser selected by the language composition.
pub(crate) trait BraceFormatSyntaxPort {
    /// Exact identity of this decoded brace grammar.
    const GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity;
    /// Parse a decoded Python string-format input.
    fn parse(input: crate::domain::format_syntax::types::FormatInput<'_>) -> BraceFormatResult;
}
