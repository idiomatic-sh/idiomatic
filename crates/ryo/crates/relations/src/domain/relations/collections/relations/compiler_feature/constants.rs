//! Shared vocabulary for language-owned file compiler modes.
use super::item::CompilerFeatureColumn;
/// Canonical query relation name.
#[must_use]
pub fn name() -> &'static str {
    "compiler-feature"
}
/// Meaning of an explicit compiler feature row.
#[must_use]
pub fn summary() -> &'static str {
    "Explicit known file-wide compiler-language feature states; absent or unsupported capability is not an enabled=false row."
}
/// Public name of each typed column.
#[must_use]
pub fn column_name(column: CompilerFeatureColumn) -> &'static str {
    match column {
        CompilerFeatureColumn::File => "file",
        CompilerFeatureColumn::Feature => "feature",
        CompilerFeatureColumn::Enabled => "enabled",
    }
}
