//! The `project` contract's own cases, beside the engine's conformance: the engine's rows run through it, it names the package
//! manifests and refuses none of these, and it links the files only when a relation of the links is asked for.

use std::sync::Arc;

use ryo::languages::{self, Project as _, contract};

use super::contract::ProjectContract;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::projects::{IMPORTS, adapters, request};
use crate::outbound::notes::factory::postcard;

/// Whether the run asks for a relation of the links between the files.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// It asks for `source_dependency`.
    Links,
    /// It asks for none.
    Nothing,
}

/// The files of a repository, each with its path and its text, and what the run asks for.
type Input = (&'static [(&'static str, &'static str)], Asked);
/// How many manifests it names, how many it refused and how many files it examined.
type Value = (usize, usize, usize);
type Failure = languages::ProjectError;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("names_the_package_manifests_and_refuses_none"),
        word: Word::Succeeds,
        input: || (&[], Asked::Nothing),
        expect: Expect::Value((2, 0, 0)),
    },
    Case {
        name: CaseName("the_files_are_examined_when_a_relation_of_the_links_is_asked_for"),
        word: Word::Succeeds,
        input: || (IMPORTS, Asked::Links),
        expect: Expect::Value((2, 0, 6)),
    },
    Case {
        name: CaseName("the_files_are_not_linked_when_no_relation_of_the_links_is_asked_for"),
        word: Word::Succeeds,
        input: || (IMPORTS, Asked::Nothing),
        expect: Expect::Value((2, 0, 0)),
    },
];

/// Runs the engine's rows against the contract, then asks it to resolve a repository.
fn subject((files, asked): Input) -> Result<Value, Failure> {
    let language = ProjectContract::new(adapters(), Arc::new(postcard()));
    contract::project(&language);
    let wanted = match asked {
        Asked::Links => vec![languages::Relation::SourceDependency],
        Asked::Nothing => Vec::new(),
    };
    let resolved = language.resolve(&request(files, wanted))?;
    Ok((
        language.manifests().len(),
        resolved.refused.len(),
        resolved.derived.analyzed.len(),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
