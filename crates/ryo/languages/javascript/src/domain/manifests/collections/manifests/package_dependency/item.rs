//! Every requirement a package of the repository declares, on another of its packages or outside it.

use crate::domain::catalogs::service::{exactness, revision_of};
use crate::domain::links::service::packages_in;
use crate::domain::links::types::Linked;
use crate::domain::packages::service::{dependencies, named};
use crate::domain::packages::types::Dependency;
use crate::domain::packages::types::Projection;
use ryo::languages;

/// Answers the `package-dependency` rows of the repository's package manifests and files.
pub(crate) struct PackageDependencyItem;

ryo::reads! {
    PackageDependencyItem => languages::PackageDependency, manifests(Linked) |input, symbols|
    model projection: Projection<Dependency> = (Some(dependencies(&input.request.files, packages_in(input)))) {
        each (&projection.refused) as refused {
            partial refused.reason.0 => languages::Missing::Any;
        }
        each (&projection.rows) as dependency {
            row {
                source: value(dependency.source),
                target: value(dependency.target),
                binding: value(named(symbols, &dependency.requirement.binding)),
                package: value(named(symbols, &dependency.requirement.package)),
                kind: value(dependency.requirement.kind),
                condition: value(languages::NO_SYMBOL),
                optional: value(dependency.requirement.optional),
                origin: value(dependency.requirement.origin),
                requirement: value(languages::intern_derived(symbols, languages::ByteSlice(&[]), languages::NameText(&dependency.requirement.spec.0))),
                exact: value(exactness(&dependency.requirement.spec)),
                // A package.json git spec names a commit-ish, not said to be a branch or a tag: it is the `revision`.
                branch: value(languages::NO_SYMBOL),
                tag: value(languages::NO_SYMBOL),
                revision: value(languages::intern_derived(symbols, languages::ByteSlice(&[]), languages::NameText(revision_of(&dependency.requirement.spec, dependency.requirement.origin).0))),
                // A package.json dependency names no version of Node: `engines` is the package's own.
                interpreter: value(languages::NO_SYMBOL),
                catalog: value(languages::intern_derived(symbols, languages::ByteSlice(&[]), languages::NameText(&dependency.requirement.written_catalog.0))),
                replacement: value(languages::intern_derived(symbols, languages::ByteSlice(&[]), languages::NameText(&dependency.replacement.0))),
            }
        }
    }
}
