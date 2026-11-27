//! Query vocabulary for binding-read.
use super::item::BindingReadColumn;
use crate::domain::relations::constants::{CONTEXT, REFERENCE, SINCE_MAJOR, SINCE_MINOR};
/// Query relation name.
#[must_use]
pub fn name() -> &'static str {
    "binding-read"
}
/// The static semantic contract, independent of lint policy.
#[must_use]
pub fn summary() -> &'static str {
    "Static reference classification; definite absence, possible absence and unsupported knowledge are distinct."
}
/// Query column name.
#[must_use]
pub fn column_name(column: BindingReadColumn) -> &'static str {
    match column {
        BindingReadColumn::Reference => REFERENCE,
        BindingReadColumn::Name => "name",
        BindingReadColumn::Scope => "scope",
        BindingReadColumn::Phase => "phase",
        BindingReadColumn::Context => CONTEXT,
        BindingReadColumn::State => "state",
        BindingReadColumn::SinceMajor => SINCE_MAJOR,
        BindingReadColumn::SinceMinor => SINCE_MINOR,
        BindingReadColumn::File => "file",
        BindingReadColumn::Span => "span",
    }
}
