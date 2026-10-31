//! Query vocabulary for binding-event.
use super::item::BindingEventColumn;
/// Query relation name.
#[must_use]
pub fn name() -> &'static str {
    "binding-event"
}
/// The static semantic contract, independent of lint policy.
#[must_use]
pub fn summary() -> &'static str {
    "Ordered static name events within explicit evaluation phases."
}
/// Query column name.
#[must_use]
pub fn column_name(column: BindingEventColumn) -> &'static str {
    match column {
        BindingEventColumn::Occurrence => "occurrence",
        BindingEventColumn::Name => "name",
        BindingEventColumn::Scope => "scope",
        BindingEventColumn::Phase => "phase",
        BindingEventColumn::Kind => "kind",
        BindingEventColumn::Ordinal => "ordinal",
        BindingEventColumn::File => "file",
        BindingEventColumn::Span => "span",
    }
}
