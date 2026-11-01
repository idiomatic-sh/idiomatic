//! A parser port whose result contains no parser-library types.
use super::types::{FormatDialect, FormatParseResult};

/// Parses one decoded format string in the selected dialect.
pub(crate) trait FormatSyntax: 'static {
    /// Parses a decoded string and returns an owned summary and conversion fields.
    fn parse(input: &str, dialect: FormatDialect) -> FormatParseResult;
}
