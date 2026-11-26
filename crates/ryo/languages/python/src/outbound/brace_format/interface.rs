//! Static parser port for brace-format syntax.
use crate::domain::brace_format::types::BraceFormatResult;

/// A deterministic parser selected by the language composition.
pub(crate) trait BraceFormatSyntaxPort {
    /// Parse a decoded Python string-format input.
    fn parse(
        &self,
        input: crate::domain::format_syntax::types::FormatInput<'_>,
    ) -> BraceFormatResult;
}
