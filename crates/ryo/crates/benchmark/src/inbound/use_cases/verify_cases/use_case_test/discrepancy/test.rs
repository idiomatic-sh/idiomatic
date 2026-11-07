//! Both sides require intended behavior even when a wrong oracle matches the reference.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::discrepancy::types::IntentAgreement;
use crate::inbound::use_cases::verify_cases::fixtures::{
    discrepancy::discrepancy_suite,
    shard::{Refusal, category, history_len, succeeding, verify_shard},
};
use crate::outbound::{
    execution::fixtures::fake::ExecutionFake,
    linter::fixtures::{covered::CoveredCandidate, fake::LinterFake},
    receipts::fixtures::fake::ReceiptsFake, suite::fixtures::fake::SuiteFake,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::path::Path;

/// Which candidate answers the documented reference difference.
#[derive(Clone, Copy)]
enum Candidate {
    Intended,
    WrongOracle,
}

type Input = Candidate;
type Value = usize;
type Failure = Refusal;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("both sides require intended behavior even when a wrong oracle matches"),
        word: Word::Succeeds,
        input: || Candidate::Intended,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("agreement with an incorrect oracle fails the intended contract"),
        word: Word::Refuses,
        input: || Candidate::WrongOracle,
        expect: Expect::Error(Refusal::CandidateMismatch),
    },
];

fn subject(candidate: Input) -> Result<Value, Failure> {
    match candidate {
        Candidate::Intended => native_intended(),
        Candidate::WrongOracle => native_wrong_oracle(),
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

/// Paired verification of the discrepancy fixture against the plain reference and `intended`.
fn native_pair(
    suite: &SuiteFake,
    intended: &CoveredCandidate,
    execution: &ExecutionFake,
) -> Result<Value, Error> {
    verify_shard(
        suite,
        &ReceiptsFake::default(),
        &LinterFake::default(),
        intended,
        execution,
        Path::new("/profile/rules"),
    )
}

fn native_intended() -> Result<Value, Error> {
    let suite = discrepancy_suite();
    let intended = CoveredCandidate(LinterFake {
        diagnostics: suite.prepared[0].expected.diagnostics.clone(),
        ..Default::default()
    });
    let execution = succeeding();
    let result = native_pair(&suite, &intended, &execution);
    assert_eq!(history_len(&execution), Some(2), "both actual sides executed");
    let mut canonical = suite.prepared.clone();
    crate::application::discrepancy_attachment::service::attach(&suite, Path::new("/profile/rules"), &mut canonical)
        .expect("application owns wire conversion");
    assert!(
        crate::domain::discrepancy::service::matches_intended(
            &canonical[0],
            crate::domain::discrepancy::service::reference_expected(&canonical[0])
                .expect("diagnostic expectation")
        ) == IntentAgreement::Differs,
        "JSONL retains raw reference mismatch"
    );
    result
}

fn native_wrong_oracle() -> Result<Value, Error> {
    let suite = discrepancy_suite();
    let wrong_oracle = CoveredCandidate(LinterFake::default());
    native_pair(&suite, &wrong_oracle, &succeeding())
}
