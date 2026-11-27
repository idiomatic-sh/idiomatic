//! Public names of primary input disk read failures.
use super::item::FilesystemReadFailureColumn;
/// The relation name.
#[must_use]
pub fn name() -> &'static str {
    "filesystem.read-failure"
}
/// The relation meaning.
#[must_use]
pub fn summary() -> &'static str {
    "An inventoried primary path whose discovery or disk read failed; no source or syntax is implied."
}
/// A public column name.
#[must_use]
pub fn column_name(column: FilesystemReadFailureColumn) -> &'static str {
    match column {
        FilesystemReadFailureColumn::Path => "path",
        FilesystemReadFailureColumn::Detail => "detail",
        FilesystemReadFailureColumn::Span => "span",
    }
}
