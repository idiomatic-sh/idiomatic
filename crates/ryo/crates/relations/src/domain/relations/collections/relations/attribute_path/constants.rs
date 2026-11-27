//! The names of the `attribute-path` relation: the text its name and its columns carry when they leave the engine.
use super::item::AttributePathColumn;

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "attribute-path"
}

/// What the relation holds, in a sentence.
#[must_use]
pub(crate) fn summary() -> &'static str {
    "Every path an attribute writes, as one name: its segments as written, joined by `::`, with comments and whitespace left out."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub(crate) fn column_name(column: AttributePathColumn) -> &'static str {
    match column {
        AttributePathColumn::Id => "id",
        AttributePathColumn::Name => "name",
        AttributePathColumn::File => "file",
        AttributePathColumn::Span => "span",
    }
}
