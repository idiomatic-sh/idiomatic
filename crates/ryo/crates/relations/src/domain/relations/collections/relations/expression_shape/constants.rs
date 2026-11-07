//! Structural expression identity vocabulary.
use super::item::ExpressionShapeColumn;
/// Query relation name.
#[must_use]
pub fn name() -> &'static str {
    "expression-shape"
}
/// AST structure is neither binding identity nor runtime value equivalence.
#[must_use]
pub fn summary() -> &'static str {
    "Collision-safe file-local parsed expression identity: ignore source positions, trivia, redundant parentheses and read/write context; retain operators, semantic fields, child order within fields, canonical identifiers and typed parsed literal payloads. Unsupported expressions have no known class and are explicitly partial."
}
/// Column name.
#[must_use]
pub fn column_name(column: ExpressionShapeColumn) -> &'static str {
    match column {
        ExpressionShapeColumn::Id => "id",
        ExpressionShapeColumn::Shape => "shape",
        ExpressionShapeColumn::File => "file",
        ExpressionShapeColumn::Span => "span",
    }
}
