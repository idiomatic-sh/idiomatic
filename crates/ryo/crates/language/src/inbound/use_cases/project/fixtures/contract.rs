//! What every implementation of the `project` use case's trait must pass.

use super::super::errors::ProjectError;
use super::super::interface::ProjectUseCase;
use super::super::service::project;
use super::super::types::{ProjectRequest, RootPath};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::symbols::types::Symbols;
use crate::inbound::use_cases::parse::fixtures::collect::Collect;
use ryo_relations::domain::relations::service::{all, class_of};
use ryo_relations::domain::relations::types::{Class, Relation};

/// What a row of the contract is given: nothing, a repository without manifests.
type Input = ();
/// How many rows the language wrote, and how many manifests it refused.
type Value = (usize, usize);
/// What a row of the contract expects when it is refused.
type Failure = ProjectError;

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("language_without_manifests_answers_an_empty_report"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value((0, 0)),
}];

/// Every relation read from the manifests: a language is asked for all of them, so that a reader that writes a row for an empty project is found.
fn manifest_relations() -> Vec<Relation> {
    all()
        .iter()
        .copied()
        .filter(|relation| class_of(*relation) == Class::Manifest)
        .collect()
}

/// Runs the contract's rows against one implementation, in the crate that implements the trait.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(subject: &impl ProjectUseCase) {
    run(CASES, Word::Succeeds, |()| {
        let mut out = Collect::default();
        project(
            subject,
            &ProjectRequest {
                search_scope: Default::default(),
                given_inventory: Default::default(),
                observed_entries: Default::default(),
                repository: crate::inbound::use_cases::project::types::Repository::Disk,
                root: RootPath(Box::default()),
                manifests: Vec::new(),
                files: Vec::new(),
                notes: Vec::new(),
                wanted: manifest_relations(),
            },
            &mut Symbols::default(),
            &mut out,
        )
        .map(|resolved| (out.relations.len(), resolved.refused.len()))
    });
}
