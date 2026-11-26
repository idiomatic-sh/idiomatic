//! Call the pinned Ruff string-annotation parser without retaining its AST.
use crate::domain::literal_expression_parsing::types::{
    CarrierInput, LiteralDialect, LiteralParseOutcome,
};
use crate::outbound::literal_expression::interface::LiteralExpressionSyntaxPort;

/// The parser used by the Python relation's static composition.
pub(crate) struct RuffStringAnnotation;

impl LiteralExpressionSyntaxPort for RuffStringAnnotation {
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

/// SPIKE (track X): what `RuffStringAnnotation::parse` answers of a carrier of one string, for a string literal the
/// file's own tree holds, read where it is written in `source`: the string-annotation grammar asked once, with no
/// carrier parsed first to get the literal's node. The grammar reads a single part whose text is its content in
/// place, and any other string by its value: neither depends on what stands around the string.
pub(crate) fn parsed_in_place(
    literal: &ruff_python_ast::ExprStringLiteral,
    source: &str,
) -> LiteralParseOutcome {
    if ruff_python_parser::typing::parse_type_annotation(literal, source).is_ok() {
        LiteralParseOutcome::Parsed
    } else {
        LiteralParseOutcome::Malformed
    }
}
