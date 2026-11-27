//! Query vocabulary for binding-shadow.
use super::item::BindingShadowColumn;
use crate::domain::relations::constants::BINDING;
/// Query relation name.
#[must_use]
pub fn name() -> &'static str {
    "binding-shadow"
}
/// The static semantic contract, independent of lint policy.
#[must_use]
pub fn summary() -> &'static str {
    "Concrete replacement and lexical-shadow associations between distinct declaration occurrences."
}
/// Query column name.
#[must_use]
pub fn column_name(column: BindingShadowColumn) -> &'static str {
    match column {
        BindingShadowColumn::Binding => BINDING,
        BindingShadowColumn::Previous => "previous",
        BindingShadowColumn::Scope => "scope",
        BindingShadowColumn::PreviousScope => "previous-scope",
        BindingShadowColumn::Phase => "phase",
        BindingShadowColumn::Relationship => "relationship",
        BindingShadowColumn::File => "file",
    }
}
