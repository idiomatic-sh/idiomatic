//! A parser port whose result contains no parser-library types.
use crate::domain::format_syntax::types::{
    FormatDialect, FormatInput, FormatParseResult, GrammarIdentity,
};

/// Parses one caller-selected format input in the selected dialect.
pub(crate) trait FormatSyntaxPort: 'static {
    /// Exact identity of decoded-string grammar results.
    const DECODED_GRAMMAR: GrammarIdentity;
    /// Exact identity of individual raw-token grammar results.
    const RAW_GRAMMAR: GrammarIdentity;
    /// Returns an owned summary and conversion fields without interpreting Python escapes.
    fn parse(input: FormatInput<'_>, dialect: FormatDialect) -> FormatParseResult;
}
