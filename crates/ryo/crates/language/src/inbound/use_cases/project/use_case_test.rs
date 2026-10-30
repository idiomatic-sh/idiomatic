//! What the `project` use case promises: the contract, run against the language without manifests, and what the service does with a
//! language whose tools cannot run.

use super::errors::ProjectError;
use super::fixtures::contract;
use super::interface::ProjectUseCase;
use super::types::ProjectEngine;
use super::types::{ManifestName, ProjectRequest, Resolved, RootPath};
use super::use_case::ProjectUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::reasons::types::ToolMessage;
use crate::domain::relations::types::Count;
use crate::domain::symbols::types::Symbols;
use crate::inbound::use_cases::parse::fixtures::contract::Discard;

/// A language without manifest relations whose tools cannot run at all.
struct Broken;

impl ProjectUseCase for Broken {
    type Derived = ();

    fn manifests(&self) -> &'static [ManifestName] {
        &[]
    }

    fn resolve(&self, _request: &ProjectRequest) -> Result<Resolved, ProjectError> {
        Err(ProjectError::Manifest {
            reason: ToolMessage("the tool could not run".into()),
        })
    }

    crate::each_relation!(crate::absent_manifests);
}

/// What a row of the cases is given.
#[derive(Clone, Copy)]
enum Scenario {
    /// The language without manifests reads a project.
    Nothing,
    /// A language whose tools cannot run reads a project.
    Broken,
}

/// What a row is given.
type Input = Scenario;
/// How many manifests the language refused.
type Value = Count;
type Failure = ProjectError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_language_without_manifests_refuses_none"),
        word: Word::Succeeds,
        input: || Scenario::Nothing,
        expect: Expect::Value(Count(0)),
    },
    Case {
        name: CaseName("a_language_whose_tools_cannot_run_stops_the_project"),
        word: Word::Refuses,
        input: || Scenario::Broken,
        expect: Expect::Refusal(|error| {
            selects(matches!(error, ProjectError::Manifest { reason }
                if &*reason.0 == "the tool could not run"))
        }),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let request = ProjectRequest {
        repository: crate::inbound::use_cases::project::types::Repository::Disk,
        root: RootPath(".".into()),
        manifests: Vec::new(),
        files: Vec::new(),
        notes: Vec::new(),
        wanted: Vec::new(),
    };
    let language: &dyn ProjectEngine = match scenario {
        Scenario::Nothing => &ProjectUseCaseImpl,
        Scenario::Broken => &Broken,
    };
    language
        .project(&request, &mut Symbols::default(), &mut Discard)
        .map(|read| Count(read.refused.len()))
}

#[test]
fn succeeds() {
    contract::check(&ProjectUseCaseImpl);
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
