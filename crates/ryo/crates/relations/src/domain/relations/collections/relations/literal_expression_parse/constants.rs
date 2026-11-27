//! Public vocabulary for literal-expression-parse.
use super::item::LiteralExpressionParseColumn;
use crate::domain::relations::constants::{DIALECT, OUTCOME};
/// Return the relation's query name.
#[must_use]
pub fn name() -> &'static str {
    "literal-expression-parse"
}
/// Describe syntactic knowledge without imposing type-position policy.
#[must_use]
pub fn summary() -> &'static str {
    "Syntactic parse outcomes for original string carriers, independent of type position."
}
/// Return a column's canonical query name.
#[must_use]
pub fn column_name(column: LiteralExpressionParseColumn) -> &'static str {
    match column {
        LiteralExpressionParseColumn::Id => "id",
        LiteralExpressionParseColumn::Dialect => DIALECT,
        LiteralExpressionParseColumn::Outcome => OUTCOME,
        LiteralExpressionParseColumn::File => "file",
        LiteralExpressionParseColumn::Span => "span",
    }
}
