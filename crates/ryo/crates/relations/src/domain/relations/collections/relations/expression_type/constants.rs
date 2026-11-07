use super::item::ExpressionTypeColumn;
/// The RyoQL relation name.
#[must_use]
pub fn name() -> &'static str {
    "expression-type"
}
/// A short description for relation listings.
#[must_use]
pub fn summary() -> &'static str {
    "Optional basic parser-derived types of individual expressions."
}
/// The RyoQL spelling of a typed column.
#[must_use]
pub fn column_name(column: ExpressionTypeColumn) -> &'static str {
    match column {
        ExpressionTypeColumn::Id => "id",
        ExpressionTypeColumn::Resolution => "resolution",
        ExpressionTypeColumn::Type => "type",
        ExpressionTypeColumn::File => "file",
        ExpressionTypeColumn::Span => "span",
    }
}
