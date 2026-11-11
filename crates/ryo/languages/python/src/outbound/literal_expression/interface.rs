//! The static parser port; parser-library types never enter the domain cache.
use crate::domain::literal_expression_parsing::types::{
    CarrierInput, LiteralDialect, LiteralParseOutcome,
};

/// Parse a reconstructed carrier and discard all temporary syntax trees.
pub(crate) trait LiteralExpressionSyntaxPort: 'static {
    /// Exact identity of this literal-expression grammar.
    const GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity;
    /// The input preserves complete original token spellings inside grouping parentheses.
    fn parse(carrier: CarrierInput<'_>, dialect: LiteralDialect) -> LiteralParseOutcome;
}
