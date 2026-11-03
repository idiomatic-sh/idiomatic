//! Public vocabulary of the embedded-expression relation.
use super::item::EmbeddedExpressionColumn;
use crate::domain::relations::collections::relations::mapping::constants::name as mapping_name;
/// The relation name exposed to queries.
#[must_use]
pub fn name() -> &'static str {
    "embedded-expression"
}
/// The relation's meaning.
#[must_use]
pub fn summary() -> &'static str {
    "A parsed semantic expression and its physical source carrier, context, and source-map fidelity."
}
/// Names of the relation's columns.
#[must_use]
pub fn column_name(column: EmbeddedExpressionColumn) -> &'static str {
    match column {
        EmbeddedExpressionColumn::Root => "root",
        EmbeddedExpressionColumn::Host => "host",
        EmbeddedExpressionColumn::Context => "context",
        EmbeddedExpressionColumn::Mapping => mapping_name(),
        EmbeddedExpressionColumn::File => "file",
        EmbeddedExpressionColumn::Span => "span",
    }
}
