//! Query vocabulary for binding-lookup.
use super::item::BindingLookupColumn;
use crate::domain::relations::constants::{CONTEXT, KIND, ORDINAL, REFERENCE};
/// Query relation name.
#[must_use]
pub fn name() -> &'static str {
    "binding-lookup"
}
/// The static semantic contract, independent of lint policy.
#[must_use]
pub fn summary() -> &'static str {
    "For each read or deletion of a name and each namespace on its Python lookup path, separately for each code block that stores into the namespace (its own block, and each block that declares the name global or nonlocal): the binding in effect when the file's schedule runs in order (the latest bind before the reference whose exception handler has not ended), and each later annotation, deletion, handler cleanup, global or nonlocal declaration of the name before the reference. When no bind precedes the reference, every such event before it."
}
/// Query column name.
#[must_use]
pub fn column_name(column: BindingLookupColumn) -> &'static str {
    match column {
        BindingLookupColumn::Reference => REFERENCE,
        BindingLookupColumn::Name => "name",
        BindingLookupColumn::Event => "event",
        BindingLookupColumn::Kind => KIND,
        BindingLookupColumn::Scope => "scope",
        BindingLookupColumn::Lexical => "lexical",
        BindingLookupColumn::Depth => "depth",
        BindingLookupColumn::Ordinal => ORDINAL,
        BindingLookupColumn::Context => CONTEXT,
        BindingLookupColumn::File => "file",
        BindingLookupColumn::Span => "span",
    }
}
