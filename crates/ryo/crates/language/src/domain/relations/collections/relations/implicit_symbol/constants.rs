//! Query vocabulary for occurrence-independent language builtins.
use super::item::ImplicitSymbolColumn;
/// Relation name.
#[must_use]
pub fn name() -> &'static str {
    "implicit-symbol"
}
/// Catalog meaning; module globals remain distinct from builtins.
#[must_use]
pub fn summary() -> &'static str {
    "Potential implicit environment names, recorded origins and numeric availability, independent of occurrences or current-host presence."
}
/// Stable public column name.
#[must_use]
pub fn column_name(column: ImplicitSymbolColumn) -> &'static str {
    match column {
        ImplicitSymbolColumn::Name => "name",
        ImplicitSymbolColumn::Kind => "kind",
        ImplicitSymbolColumn::Scope => "scope",
        ImplicitSymbolColumn::SinceMajor => "since-major",
        ImplicitSymbolColumn::SinceMinor => "since-minor",
        ImplicitSymbolColumn::File => "file",
    }
}
