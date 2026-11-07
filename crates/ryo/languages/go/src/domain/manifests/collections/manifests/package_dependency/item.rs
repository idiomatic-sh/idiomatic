//! The `package_dependency` relation of the language: each requirement of a module, on another module of the repository found by its
//! path or by the directory a `replace` sends it to, or on a module outside it, whose row has no target.

use crate::domain::links::service::modules_in;
use crate::domain::links::types::Linked;
use crate::domain::modules::constants::NO_CONDITION;
use crate::domain::modules::service::{replacement_text, wirings_in};
use crate::domain::modules::types::{Wired, Wiring};
use crate::domain::names::service::named;
use ryo::languages;

/// The reader the `project` contract hands the `package_dependency` relation to, which reads its rows from the repository's manifests.
pub(crate) struct PackageDependencyItem;

// A manifest the tool refused declares nothing, so what it would have said is missing.
ryo::reads! {
    PackageDependencyItem => languages::PackageDependency, manifests(Linked) |input, symbols|
    model wired: Wired<Wiring> = (Some(wirings_in(&input.request.files, modules_in(input)))) {
        each (&wired.refused) as refused {
            partial refused.0 => languages::Missing::Any;
        }
        each (&wired.rows) as wiring {
            row {
                source: value(wiring.source),
                target: value(wiring.target),
                binding: value(named(symbols, languages::NameText(&wiring.path.0))),
                package: value(named(symbols, languages::NameText(&wiring.path.0))),
                kind: value(languages::DependencyKind::Normal),
                condition: value(named(symbols, languages::NameText(NO_CONDITION))),
                optional: value(languages::Optionality::Required),
                origin: value(wiring.origin),
                // A `require` names the least version minimal version selection accepts: a minimum, never one version alone.
                requirement: value(named(symbols, languages::NameText(&wiring.version.0))),
                exact: value(languages::Flag::Lowered),
                // go.mod names module versions only: a commit is reached through a pseudo-version, which `requirement` holds.
                branch: value(languages::NO_SYMBOL),
                tag: value(languages::NO_SYMBOL),
                revision: value(languages::NO_SYMBOL),
                interpreter: value(languages::NO_SYMBOL),
                catalog: value(languages::NO_SYMBOL),
                replacement: value(named(symbols, languages::NameText(&replacement_text(wiring.replacement.as_ref()).0))),
            }
        }
    }
}
