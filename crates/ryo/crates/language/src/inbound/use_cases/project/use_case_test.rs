//! What the `project` use case promises: the contract, run against the language without manifests, and what the service does with a
//! language whose tools cannot run.

use super::errors::ProjectError;
use super::fixtures::contract;
use super::interface::ProjectUseCase;
use super::types::ProjectEngine;
use super::types::{ProjectRequest, Resolved, RootPath};
use super::use_case::ProjectUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::files::types::ManifestName;
use crate::domain::reasons::types::ToolMessage;
use crate::domain::symbols::types::Symbols;
use crate::inbound::use_cases::parse::fixtures::contract::Discard;
use crate::selects;
use ryo_relations::domain::relations::types::Count;

/// A language without manifest relations whose project is refused with the failure it holds.
struct Broken(ProjectError);

impl ProjectUseCase for Broken {
    type Derived = ();

    fn manifests(&self) -> &'static [ManifestName] {
        &[]
    }

    fn primary_role(
        &self,
        _path: crate::domain::files::types::SourcePath<'_>,
    ) -> crate::domain::files::types::PrimaryFileRole {
        crate::domain::files::service::primary_role_from_manifests(
            _path,
            self.manifests().iter().copied(),
        )
    }

    fn project_entries(
        &self,
        _request: &ProjectRequest,
        _entries: &mut Vec<crate::domain::files::types::FilePath>,
    ) {
    }

    fn resolve(&self, _request: &ProjectRequest) -> Result<Resolved, ProjectError> {
        Err(self.0.clone())
    }

    ryo_relations::each_relation!(absent_manifests);
}

/// What a row of the cases is given.
#[derive(Clone, Copy)]
enum Scenario {
    /// The language without manifests reads a project.
    Nothing,
    /// A language whose tools cannot run reads a project.
    Broken,
    /// A language whose repository context cannot be established reads a project.
    Unlocated,
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
            selects!(error, ProjectError::Manifest { reason }
                if &*reason.0 == "the tool could not run")
        }),
    },
    Case {
        name: CaseName("a_language_without_repository_context_stops_the_project"),
        word: Word::Refuses,
        input: || Scenario::Unlocated,
        expect: Expect::Refusal(|error| {
            selects!(error, ProjectError::Context { reason }
                if &*reason.0 == "the repository was not found")
        }),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let request = ProjectRequest {
        search_scope: Default::default(),
        given_inventory: Default::default(),
        observed_entries: Default::default(),
        repository: crate::inbound::use_cases::project::types::Repository::Disk,
        root: RootPath(".".into()),
        manifests: Vec::new(),
        files: Vec::new(),
        notes: Vec::new(),
        wanted: Vec::new(),
    };
    let broken = Broken(match scenario {
        Scenario::Broken => ProjectError::Manifest {
            reason: ToolMessage("the tool could not run".into()),
        },
        _ => ProjectError::Context {
            reason: ToolMessage("the repository was not found".into()),
        },
    });
    let language: &dyn ProjectEngine = match scenario {
        Scenario::Nothing => &ProjectUseCaseImpl,
        Scenario::Broken | Scenario::Unlocated => &broken,
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
