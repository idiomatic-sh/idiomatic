//! Query vocabulary for binding-use.
use super::item::BindingUseColumn;
/// Query relation name.
#[must_use]
pub fn name() -> &'static str {
    "binding-use"
}
/// The static semantic contract, independent of lint policy.
#[must_use]
pub fn summary() -> &'static str {
    "Known real declaration candidates for a semantic reference; unresolved references have no fake target edge."
}
/// Query column name.
#[must_use]
pub fn column_name(column: BindingUseColumn) -> &'static str {
    match column {
        BindingUseColumn::Reference => "reference",
        BindingUseColumn::Binding => "binding",
        BindingUseColumn::Scope => "scope",
        BindingUseColumn::Phase => "phase",
        BindingUseColumn::Context => "context",
        BindingUseColumn::Certainty => "certainty",
        BindingUseColumn::File => "file",
        BindingUseColumn::Span => "span",
    }
}
