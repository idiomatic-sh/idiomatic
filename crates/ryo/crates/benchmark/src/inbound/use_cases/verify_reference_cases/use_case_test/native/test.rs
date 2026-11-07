//! Reference-only verification cannot bypass the pinned reference expectation or its evidence.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::verify_cases::use_case_test::{
    discrepancy::fixtures::discrepancy_suite,
    shard::fixtures::{Refusal, category, succeeding},
};
use crate::inbound::use_cases::verify_reference_cases::service::verify_prepared_reference_cases;
use crate::outbound::{linter::fixtures::fake::LinterFake, suite::fixtures::fake::SuiteFake, execution::fixtures::fake::ExecutionFake};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::path::Path;

/// What the pinned reference answers against the documented expectation.
#[derive(Clone, Copy)]
enum Scenario {
    Pinned,
    IntendedInsteadOfPinned,
    StaleEvidence,
}

type Input = Scenario;
type Value = usize;
type Failure = Refusal;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("reference only checks the distinct pinned reference expectation"),
        word: Word::Succeeds,
        input: || Scenario::Pinned,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("intended candidate output cannot replace pinned reference behavior"),
        word: Word::Refuses,
        input: || Scenario::IntendedInsteadOfPinned,
        expect: Expect::Error(Refusal::ReferenceMismatch),
    },
    Case {
        name: CaseName("reference only cannot bypass independent evidence"),
        word: Word::Refuses,
        input: || Scenario::StaleEvidence,
        expect: Expect::Error(Refusal::UnprovenEvidence),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::Pinned => pinned(),
        Scenario::IntendedInsteadOfPinned => intended_instead_of_pinned(),
        Scenario::StaleEvidence => stale_evidence(),
    }
    .map_err(|error| category(&error.to_string()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

/// The reference-only verification of the discrepancy fixture.
fn reference_only(
    suite: &SuiteFake,
    reference: &LinterFake,
    execution: &ExecutionFake,
) -> Result<Value, Error> {
    verify_prepared_reference_cases(
        suite,
        &crate::outbound::context::factory::local(),
        reference,
        execution,
        &crate::outbound::inventory::factory::local(),
        Path::new("/profile/rules"),
        suite.prepared.clone(),
    )
    .map(|count| count.0)
}

fn pinned() -> Result<Value, Error> {
    let suite = discrepancy_suite();
    reference_only(&suite, &LinterFake::default(), &succeeding())
}

fn intended_instead_of_pinned() -> Result<Value, Error> {
    let suite = discrepancy_suite();
    let wrong_reference = LinterFake {
        diagnostics: suite.prepared[0].expected.diagnostics.clone(),
        ..Default::default()
    };
    reference_only(&suite, &wrong_reference, &succeeding())
}

fn stale_evidence() -> Result<Value, Error> {
    let mut stale = discrepancy_suite();
    stale.discrepancy_artifacts_valid = false;
    reference_only(&stale, &LinterFake::default(), &succeeding())
}
