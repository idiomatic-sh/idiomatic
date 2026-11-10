//! The names of the `parameter` relation: the text its name and its columns carry when they leave the engine.

use super::item::ParameterColumn;
use crate::domain::relations::constants::ORDINAL;

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    crate::domain::terms::constants::PARAMETER_OWNER.0
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "A parameter of a function, method or closure."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: ParameterColumn) -> &'static str {
    match column {
        ParameterColumn::Id => "id",
        ParameterColumn::Kind => "kind",
        ParameterColumn::Refinement => "refinement",
        ParameterColumn::Name => "name",
        ParameterColumn::Ordinal => ORDINAL,
        ParameterColumn::Owner => "owner",
        ParameterColumn::File => "file",
        ParameterColumn::Span => "span",
    }
}
