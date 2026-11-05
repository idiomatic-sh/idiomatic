//! The `package_dependency` relation: one row for each dependency a package declares.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Flag, Optionality};
use crate::domain::terms::service::term_symbol;
use crate::domain::terms::types::{DependencyKind, DependencyOrigin, Symbol};

/// `package_dependency`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct PackageDependency;

impl super::super::interface::sealed::Sealed for PackageDependency {}

/// The declaration of `package_dependency`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! package_dependency {
    ($callback:path) => {
        $callback! {
            item PackageDependency,
            row {
                /// One row of `package_dependency`.
                PackageDependencyRow
            },
            column {
                /// The columns of `package_dependency`.
                PackageDependencyColumn
            },
            expected {
                /// One row of `package_dependency` as a fixture states it.
                PackageDependencyExpectedRow
            },
            class Manifest,
            columns {
                /// The package that depends: its manifest.
                source Source: FileId => File;
                /// The package it depends on: its manifest, when the repository holds it; `NO_FILE`, shown empty, when it does not (a package
                /// from a registry, a git repository, an archive or a directory outside the repository).
                target Target: FileId => File;
                /// The name the dependency is bound to.
                binding Binding: Symbol => Symbol;
                /// The name of the package depended on.
                package Package: Symbol => Symbol;
                /// How it is depended on.
                kind Kind: DependencyKind => Symbol(term_symbol);
                /// What must hold for the dependency to apply, empty when nothing. Source text: a platform expression as the manifest writes it.
                condition Condition: Symbol => Symbol;
                /// Whether it is needed only when asked for.
                optional Optional: Optionality => Flag(optionality_flag);
                /// Where the manifest says the package comes from: `registry`, `path`, `git`, `url` or `workspace`.
                origin Origin: DependencyOrigin => Symbol(term_symbol);
                /// The version requirement as the manifest writes it, empty when it writes none (a dependency on a path or a git revision alone,
                /// or a name with no version). Source text: Cargo's `=1.2.3`, npm's `^4.6.0`, PEP 508's `==0.115.0`, go.mod's `v1.4.0`, each as
                /// written, operators included.
                requirement Requirement: Symbol => Symbol;
                /// Whether the requirement admits one version only: a resolver that honours it installs that version or fails. Raised for an
                /// exact pin in the ecosystem's own grammar (Cargo's `=1.2.3`, npm's `1.2.3`, PEP 440's `==1.2.3` or `===1.2.3`, Poetry's
                /// `1.2.3`); lowered for a range, a wildcard, a minimum (a go.mod `require`, which minimal version selection may raise), and for
                /// a dependency with no requirement.
                exact Exact: Flag => Flag;
                /// The branch of a git repository the dependency follows, empty when it names none. Source text: as the manifest writes it.
                branch Branch: Symbol => Symbol;
                /// The tag of a git repository the dependency is taken at, empty when it names none. Source text: as the manifest writes it.
                tag Tag: Symbol => Symbol;
                /// A revision of a git repository the dependency is taken at, not said to be a branch or a tag, empty when it names none. Source
                /// text: Cargo's `rev`, Pipenv's `ref`, the revision of a git URL, as the manifest writes it.
                revision Revision: Symbol => Symbol;
                /// The versions of the interpreter the dependency applies under, empty when it writes none. Source text: the interpreter's
                /// requirement, as written.
                interpreter Interpreter: Symbol => Symbol;
                /// The shared table the requirement is taken from instead of being written, empty when it is written. Source text: `catalog:`,
                /// `catalog:<name>`, `workspace = true`, as the manifest spells it.
                catalog Catalog: Symbol => Symbol;
                /// What the manifest puts in the place of the package depended on, empty when nothing. Source text: a go.mod `replace` target
                /// with its version, a `[patch]` directory, an `overrides` spec, as written.
                replacement Replacement: Symbol => Symbol;
            }
        }
    };
}

crate::package_dependency!(crate::relation_row);

/// Whether a dependency is needed only when asked for, as the store holds it.
fn optionality_flag(optionality: Optionality) -> Flag {
    match optionality {
        Optionality::Optional => Flag::Raised,
        Optionality::Required => Flag::Lowered,
    }
}
