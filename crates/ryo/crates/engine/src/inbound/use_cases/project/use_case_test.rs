//! What the `project` use case promises: a rule set with no cross-file program finds nothing after the files.

use super::errors::ProjectPassError;
use super::fixtures::{Found, NoProject, OneFile};
use super::interface::ProjectUseCase as _;
use super::use_case::ProjectUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::findings::types::{InstructionCount, ScanCounters};
use crate::domain::programs::service::scratch_for;
use crate::domain::programs::types::Programs;
use ryo_language::domain::facts::types::ProjectFacts;
use ryo_relations::domain::relations::types::Count;

type Input = ();
/// The counters and the rows found.
type Value = (ScanCounters, Count);
type Failure = ProjectPassError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_rule_set_with_no_cross_file_program_finds_nothing"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value((
        ScanCounters {
            steps: InstructionCount(0),
            candidates: InstructionCount(0),
            made_rows: InstructionCount(0),
            fill_rows: InstructionCount(0),
            visited: InstructionCount(0),
        },
        Count(0),
    )),
}];

fn subject((): Input) -> Result<Value, Failure> {
    let programs = Programs::default();
    let mut found = Found::default();
    let facts = ProjectFacts {
        facts: &mut NoProject,
    };
    let counters = ProjectUseCaseImpl::new().project(
        &programs,
        &OneFile { empty: true },
        facts,
        &mut scratch_for(&programs),
        &mut found,
    )?;
    Ok((counters, found.0))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
