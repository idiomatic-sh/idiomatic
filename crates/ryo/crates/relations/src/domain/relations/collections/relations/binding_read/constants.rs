//! Query vocabulary for binding-read.
use super::item::BindingReadColumn;
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
        BindingReadColumn::Reference => "reference",
        BindingReadColumn::Name => "name",
        BindingReadColumn::Scope => "scope",
        BindingReadColumn::Phase => "phase",
        BindingReadColumn::Context => "context",
        BindingReadColumn::State => "state",
        BindingReadColumn::SinceMajor => "since-major",
        BindingReadColumn::SinceMinor => "since-minor",
        BindingReadColumn::File => "file",
        BindingReadColumn::Span => "span",
    }
}
