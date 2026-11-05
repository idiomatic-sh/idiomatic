//! The static parser port; parser-library types never enter the domain cache.
use super::types::{LiteralDialect, LiteralParseOutcome};

/// Parse a reconstructed carrier and discard all temporary syntax trees.
pub(crate) trait LiteralExpressionSyntax: 'static {
    /// The input preserves complete original token spellings inside grouping parentheses.
    fn parse(carrier: &str, dialect: LiteralDialect) -> LiteralParseOutcome;
}
