//! Call the pinned Ruff string-annotation parser without retaining its AST.
use crate::domain::literal_expression_parsing::types::{
    CarrierInput, LiteralDialect, LiteralParseOutcome,
};
use crate::outbound::literal_expression::interface::LiteralExpressionSyntaxPort;

/// The parser used by the Python relation's static composition.
pub(crate) struct RuffStringAnnotation;

impl LiteralExpressionSyntaxPort for RuffStringAnnotation {
    const GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::literal_expression_parsing::constants::GRAMMAR;

    fn parse(&self, carrier: CarrierInput<'_>, dialect: LiteralDialect) -> LiteralParseOutcome {
        match dialect {
            LiteralDialect::PythonStringAnnotation => {}
        }
        let Ok(parsed) = ruff_python_parser::parse_expression(carrier.0) else {
            return LiteralParseOutcome::Unavailable;
        };
        let ruff_python_ast::Expr::StringLiteral(literal) = parsed.expr() else {
            return LiteralParseOutcome::Unavailable;
        };
        if ruff_python_parser::typing::parse_type_annotation(literal, carrier.0).is_ok() {
            LiteralParseOutcome::Parsed
        } else {
            LiteralParseOutcome::Malformed
        }
    }
}
