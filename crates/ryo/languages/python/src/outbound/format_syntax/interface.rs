//! A parser port whose result contains no parser-library types.
use crate::domain::format_syntax::types::{FormatDialect, FormatInput, FormatParseResult};

/// Parses one caller-selected format input in the selected dialect.
pub(crate) trait FormatSyntaxPort: 'static {
    /// Returns an owned summary and conversion fields without interpreting Python escapes.
    fn parse(&self, input: FormatInput<'_>, dialect: FormatDialect) -> FormatParseResult;
}
