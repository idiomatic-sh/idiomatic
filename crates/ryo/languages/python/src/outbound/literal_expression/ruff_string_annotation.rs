//! Call the pinned Ruff string-annotation parser without retaining its AST.
use crate::domain::literal_expression_parsing::{
    interface::LiteralExpressionSyntax,
    types::{LiteralDialect, LiteralParseOutcome},
};

/// The parser used by the Python relation's static composition.
pub(crate) struct RuffStringAnnotation;

impl LiteralExpressionSyntax for RuffStringAnnotation {
    const GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::literal_expression_parsing::constants::GRAMMAR;

    fn parse(carrier: &str, dialect: LiteralDialect) -> LiteralParseOutcome {
        match dialect {
            LiteralDialect::PythonStringAnnotation => {}
        }
        let Ok(parsed) = ruff_python_parser::parse_expression(carrier) else {
            return LiteralParseOutcome::Unavailable;
        };
        let ruff_python_ast::Expr::StringLiteral(literal) = parsed.expr() else {
            return LiteralParseOutcome::Unavailable;
        };
        if ruff_python_parser::typing::parse_type_annotation(literal, carrier).is_ok() {
            LiteralParseOutcome::Parsed
        } else {
            LiteralParseOutcome::Malformed
        }
    }
}
