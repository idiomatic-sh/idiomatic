//! The names of the `semantic_child` relation: the text its name and its columns carry when they leave the engine.

use super::item::SemanticChildColumn;
use crate::domain::relations::constants::{ORDINAL, PARENT};

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "semantic-child"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "An edge of the semantic syntax projection, ordered under its parent with its grammatical role."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: SemanticChildColumn) -> &'static str {
    match column {
        SemanticChildColumn::Parent => PARENT,
        SemanticChildColumn::Child => "child",
        SemanticChildColumn::Ordinal => ORDINAL,
        SemanticChildColumn::Field => "field",
        SemanticChildColumn::File => "file",
    }
}
