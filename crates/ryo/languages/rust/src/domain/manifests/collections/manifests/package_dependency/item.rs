//! The `package_dependency` relation of the language: each dependency a package of the repository declares, with the manifest of the
//! package depended on when the repository holds it.

use crate::domain::links::types::Linked;

use crate::domain::dependencies::service::{
    catalog_text, exactness, replacement_text, requirement_text, revision_text,
};
use crate::domain::link_state::service::located_of;
use crate::domain::packages::service::{is_unanalyzed, named, refusal_cause, unanalyzed_cause};
use crate::domain::packages::types::{Analysis, Located};
use ryo::languages;

/// The reader the `ryo::reads!` below binds the `package_dependency` relation's rows to.
pub(crate) struct PackageDependencyItem;

// A manifest Cargo refused declares nothing, so what it would have said is missing; a package depended on that no workspace holds was
// not read, so what it declares is missing too: that is said of its own manifest, whose rows they are.
ryo::reads! {
    PackageDependencyItem => languages::PackageDependency, manifests(Linked) |input, symbols|
    model located: Located = (input.derived.packages.as_ref().map(|packages| located_of(&input.request.files, packages))) {
        each (&located.packages.refused) as refused {
            partial refusal_cause(refused).to_string() => languages::Missing::Any;
        }
        each (&located.packages.dependencies) as dependency {
            when is_unanalyzed(&located.packages, dependency) {
                // The rows missing are the ones the package depended on would write, about its own manifest.
                Analysis::Unanalyzed => {
                    given unread = dependency.target.as_ref().and_then(|target| located.listed.get(target)).copied()?;
                    partial_at unread, unanalyzed_cause(dependency).to_string() => languages::Missing::Any;
                }
                Analysis::Analyzed => {}
            }
            given source = located.listed.get(&dependency.source).copied()?;
            given target = (match &dependency.target {
                Some(target) => located.listed.get(target).copied(),
                None => Some(languages::NO_FILE),
            })?;
            row {
                source: value(source),
                target: value(target),
                binding: value(named(symbols, languages::NameText(&dependency.binding))),
                package: value(named(symbols, languages::NameText(&dependency.package))),
                kind: value(dependency.kind),
                condition: value(named(symbols, languages::NameText(&dependency.condition))),
                optional: value(dependency.optional),
                origin: value(dependency.origin),
                requirement: value(named(symbols, requirement_text(&dependency.requirement))),
                exact: value(exactness(&dependency.requirement)),
                branch: value(named(symbols, revision_text(&dependency.checkout.branch))),
                tag: value(named(symbols, revision_text(&dependency.checkout.tag))),
                revision: value(named(symbols, revision_text(&dependency.checkout.rev))),
                // A Cargo dependency names no version of Rust: `rust-version` is the package's own.
                interpreter: value(languages::NO_SYMBOL),
                catalog: value(named(symbols, catalog_text(dependency.inheritance))),
                replacement: value(named(symbols, replacement_text(&dependency.replacement))),
            }
        }
    }
}
