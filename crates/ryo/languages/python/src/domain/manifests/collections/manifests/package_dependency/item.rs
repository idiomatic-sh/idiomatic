//! The `package_dependency` relation of the language: each requirement of a package, on another package of the repository found by
//! the normalized name or the directory, or on a package outside it, whose row has no target. A requirement that writes no name (a
//! URL or a path alone) has the unknown value of an unnamed requirement for its binding and its package.

use crate::domain::dependencies::service::wirings_in;
use crate::domain::links::service::packages_in;
use crate::domain::links::types::Linked;
use crate::domain::names::service::unknown;
use crate::domain::names::types::Unresolved;
use crate::domain::packages::service::named;
use crate::domain::packages::types::{Wired, Wiring};
use ryo::languages;

/// Answers the `package-dependency` rows of the repository's Python manifests and files.
pub(crate) struct PackageDependencyItem;

// A manifest the tool refused declares nothing, so what it would have said is missing.
ryo::reads! {
    PackageDependencyItem => languages::PackageDependency, manifests(Linked) |input, symbols|
    model wired: Wired<Wiring> = (Some(wirings_in(&input.request.files, packages_in(input)))) {
        each (&wired.refused) as refused {
            partial refused.0 => languages::Missing::Any;
        }
        each (&wired.rows) as wiring {
            row {
                source: value(wiring.source),
                target: value(wiring.target),
                binding: value(wiring.dependency.binding.as_ref().map_or(unknown(Unresolved::UnnamedRequirement), |binding| named(symbols, languages::NameText(&binding.0)))),
                package: value(wiring.dependency.package.as_ref().map_or(unknown(Unresolved::UnnamedRequirement), |package| named(symbols, languages::NameText(&package.0)))),
                kind: value(wiring.dependency.kind),
                condition: value(named(symbols, languages::NameText(&wiring.dependency.condition.0))),
                optional: value(wiring.dependency.optional),
                origin: value(wiring.dependency.origin),
                requirement: value(named(symbols, languages::NameText(&wiring.dependency.requirement.0))),
                exact: value(wiring.dependency.exact),
                branch: value(named(symbols, languages::NameText(&wiring.dependency.checkout.branch.0))),
                tag: value(named(symbols, languages::NameText(&wiring.dependency.checkout.tag.0))),
                revision: value(named(symbols, languages::NameText(&wiring.dependency.checkout.revision.0))),
                interpreter: value(named(symbols, languages::NameText(&wiring.dependency.interpreter.0))),
                // No Python manifest takes a requirement from a shared table: uv's `workspace = true` is where the package comes from.
                catalog: value(languages::NO_SYMBOL),
                replacement: value(named(symbols, languages::NameText(&wiring.dependency.replacement.0))),
            }
        }
    }
}
