//! The names of the `decorates` relation: the text its name and its columns carry when they leave the engine.

use super::item::DecoratesColumn;
use crate::domain::relations::constants::{OCCURRENCE, ORDINAL, TARGET};

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "decorates"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "A decoration of a declaration, by the path it names and the arguments it passes, in its place."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: DecoratesColumn) -> &'static str {
    match column {
        DecoratesColumn::Occurrence => OCCURRENCE,
        DecoratesColumn::Declaration => crate::domain::relations::constants::DECLARATION,
        DecoratesColumn::Ordinal => ORDINAL,
        DecoratesColumn::Target => TARGET,
        DecoratesColumn::Arguments => "arguments",
        DecoratesColumn::File => "file",
        DecoratesColumn::Span => "span",
    }
}
