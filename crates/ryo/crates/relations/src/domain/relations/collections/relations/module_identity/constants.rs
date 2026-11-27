//! External vocabulary of the optional module identity relation.
use super::item::ModuleIdentityColumn;
/// Public relation name.
#[must_use]
pub fn name() -> &'static str {
    "module-identity"
}
/// What this project relation establishes.
#[must_use]
pub fn summary() -> &'static str {
    "The importable module identity of a source file, established from project context."
}
/// Public column spelling.
#[must_use]
pub fn column_name(column: ModuleIdentityColumn) -> &'static str {
    match column {
        ModuleIdentityColumn::File => "file",
        ModuleIdentityColumn::Path => "path",
        ModuleIdentityColumn::ModuleName => "module-name",
        ModuleIdentityColumn::PackageName => "package-name",
        ModuleIdentityColumn::PackageRootName => "package-root-name",
    }
}
