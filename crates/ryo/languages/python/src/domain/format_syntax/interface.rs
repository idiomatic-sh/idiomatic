//! A parser port whose result contains no parser-library types.
use super::types::{FormatDialect, FormatParseResult};

/// Parses one caller-selected format input in the selected dialect.
pub(crate) trait FormatSyntax: 'static {
    /// Returns an owned summary and conversion fields without interpreting Python escapes.
    fn parse(input: &str, dialect: FormatDialect) -> FormatParseResult;
}
