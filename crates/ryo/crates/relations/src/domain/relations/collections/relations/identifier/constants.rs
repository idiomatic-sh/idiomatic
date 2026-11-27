//! Public identifier vocabulary.
use super::item::IdentifierColumn;
/// Query relation name.
#[must_use]
pub fn name() -> &'static str {
    "identifier"
}
/// Normalization does not imply import or runtime identity.
#[must_use]
pub fn summary() -> &'static str {
    "Canonical lexical identifier spelling at its original syntax occurrence and source extent; independent of binding resolution."
}
/// Public column name.
#[must_use]
pub fn column_name(column: IdentifierColumn) -> &'static str {
    match column {
        IdentifierColumn::Id => "id",
        IdentifierColumn::Name => "name",
        IdentifierColumn::File => "file",
        IdentifierColumn::Span => "span",
    }
}
