//! Query vocabulary for binding-resolution.
use super::item::BindingResolutionColumn;
use crate::domain::relations::constants::{CONTEXT, KIND, ORDINAL, REFERENCE};
/// Query relation name.
#[must_use]
pub fn name() -> &'static str {
    "binding-resolution"
}
/// The static semantic contract, independent of lint policy.
#[must_use]
pub fn summary() -> &'static str {
    "For each read or deletion and each namespace on its Python lookup path that holds an event of the name before the read's evaluation point, the event in effect there when the file's schedule runs in order; alternatives are not joined."
}
/// Query column name.
#[must_use]
pub fn column_name(column: BindingResolutionColumn) -> &'static str {
    match column {
        BindingResolutionColumn::Reference => REFERENCE,
        BindingResolutionColumn::Event => "event",
        BindingResolutionColumn::Kind => KIND,
        BindingResolutionColumn::Scope => "scope",
        BindingResolutionColumn::Lexical => "lexical",
        BindingResolutionColumn::Depth => "depth",
        BindingResolutionColumn::Ordinal => ORDINAL,
        BindingResolutionColumn::Context => CONTEXT,
        BindingResolutionColumn::File => "file",
        BindingResolutionColumn::Span => "span",
    }
}
