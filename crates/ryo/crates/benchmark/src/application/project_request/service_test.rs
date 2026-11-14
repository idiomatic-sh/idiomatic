//! An isolated corpus is sent unsealed; a listed-disk corpus is sealed through the context boundary.
use super::service::request;
use super::types::ProjectCorpus;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::context::fixtures::fake::ContextFake;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::requests::types::{CohortSelection, ContextSeal, SearchRoots};
use ryo_benchmark_contracts::domain::workloads::types::RuleContextProfile;
use std::borrow::Cow;
use std::path::{Path, PathBuf};

type Input = (ProjectContextMode, &'static [&'static str]);
type Value = bool;
type Failure = Cow<'static, str>;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an isolated corpus is sent unsealed"),
        word: Word::Succeeds,
        input: || (ProjectContextMode::Isolated, &["a.py"]),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("a listed-disk corpus is sealed through the context boundary"),
        word: Word::Succeeds,
        input: || (ProjectContextMode::ListedDisk, &["a.py"]),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a listed-disk source the corpus does not track is refused"),
        word: Word::Refuses,
        input: || (ProjectContextMode::ListedDisk, &["untracked.py"]),
        expect: Expect::Error(Cow::Borrowed("sealed corpus is not unique")),
    },
];

fn subject((mode, files): Input) -> Result<Value, Failure> {
    let cohort = CohortSelection::Explicit(Vec::new());
    let files = files
        .iter()
        .map(|file| ryo::linter::RelativePath((*file).into()))
        .collect::<Vec<_>>();
    request(
        &ContextFake,
        &ProjectCorpus {
            cohort: &cohort,
            rule_context: &RuleContextProfile::Undeclared,
            search_roots: &SearchRoots::Undeclared,
            context_mode: mode,
        },
        Path::new("/corpus"),
        &files,
        &[PathBuf::from("a.py")],
        PathBuf::from("state"),
    )
    .map(|request| matches!(request.context_manifest, ContextSeal::Sealed(_)))
    .map_err(|error| Cow::Owned(error.to_string()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
