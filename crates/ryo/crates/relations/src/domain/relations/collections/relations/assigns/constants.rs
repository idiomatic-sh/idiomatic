//! The names of the `assigns` relation: the text its name and its columns carry when they leave the engine.

use super::item::AssignsColumn;
use crate::domain::relations::constants::{BINDING, OCCURRENCE};

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "assigns"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "The string literal a name is bound to, linked to the binding and the literal."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: AssignsColumn) -> &'static str {
    match column {
        AssignsColumn::Occurrence => OCCURRENCE,
        AssignsColumn::Binding => BINDING,
        AssignsColumn::Value => "value",
        AssignsColumn::Owner => "owner",
        AssignsColumn::File => "file",
        AssignsColumn::Span => "span",
    }
}
