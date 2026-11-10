//! The vocabulary of the optional `literal-properties` relation.

use super::item::LiteralPropertiesColumn;
use crate::domain::relations::constants::BOOLEAN;

/// Query relation name.
#[must_use]
pub fn name() -> &'static str {
    "literal-properties"
}

/// Primitive parser categories and only the explicitly named parsed predicates.
#[must_use]
pub fn summary() -> &'static str {
    "Optional parsed primitive kind and narrow zero, empty-string-or-bytes, and true-boolean properties; these columns do not imply general truthiness or runtime equality."
}

/// Column name.
#[must_use]
pub fn column_name(column: LiteralPropertiesColumn) -> &'static str {
    match column {
        LiteralPropertiesColumn::Id => "id",
        LiteralPropertiesColumn::Kind => "kind",
        LiteralPropertiesColumn::Zero => "zero",
        LiteralPropertiesColumn::Empty => "empty",
        LiteralPropertiesColumn::Boolean => BOOLEAN,
        LiteralPropertiesColumn::File => "file",
        LiteralPropertiesColumn::Span => "span",
    }
}
