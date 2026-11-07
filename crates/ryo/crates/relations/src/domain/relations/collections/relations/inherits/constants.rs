//! The names of the `inherits` relation: the text its name and its columns carry when they leave the engine.

use super::item::InheritsColumn;
use crate::domain::relations::constants::{OCCURRENCE, TARGET};

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "inherits"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "A base a type is declared to extend or implement, by the name it gives."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: InheritsColumn) -> &'static str {
    match column {
        InheritsColumn::Occurrence => OCCURRENCE,
        InheritsColumn::Declaration => crate::domain::relations::constants::DECLARATION,
        InheritsColumn::Target => TARGET,
        InheritsColumn::File => "file",
        InheritsColumn::Span => "span",
    }
}
