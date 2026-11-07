//! The names of the `package_dependency` relation: the text its name and its columns carry when they leave the engine.

use super::item::PackageDependencyColumn;
use crate::domain::relations::constants::{
    BINDING, BRANCH, CATALOG, INTERPRETER, ORIGIN, PACKAGE, REPLACEMENT, REVISION, SOURCE, TAG,
    TARGET,
};

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    crate::domain::relations::constants::PACKAGE_DEPENDENCY
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "A package that depends on another, with how, and the versions it admits."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: PackageDependencyColumn) -> &'static str {
    match column {
        PackageDependencyColumn::Source => SOURCE,
        PackageDependencyColumn::Target => TARGET,
        PackageDependencyColumn::Binding => BINDING,
        PackageDependencyColumn::Package => PACKAGE,
        PackageDependencyColumn::Kind => "kind",
        PackageDependencyColumn::Condition => "condition",
        PackageDependencyColumn::Optional => "optional",
        PackageDependencyColumn::Origin => ORIGIN,
        PackageDependencyColumn::Requirement => "requirement",
        PackageDependencyColumn::Exact => "exact",
        PackageDependencyColumn::Branch => BRANCH,
        PackageDependencyColumn::Tag => TAG,
        PackageDependencyColumn::Revision => REVISION,
        PackageDependencyColumn::Interpreter => INTERPRETER,
        PackageDependencyColumn::Catalog => CATALOG,
        PackageDependencyColumn::Replacement => REPLACEMENT,
    }
}
