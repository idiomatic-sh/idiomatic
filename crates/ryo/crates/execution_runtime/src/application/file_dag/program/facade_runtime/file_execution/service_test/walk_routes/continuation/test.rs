//! A source class is not a continuation result, and its consumers own immutable answers.
use super::super::fixtures::Residency;
use super::fixtures::exercise;
use crate::cases::{Case, CaseName, Expect, Word, run};

fn shared_source_continuation_failure_and_result_consumers_preserve_one_certified_population() {
    exercise(Residency::Stable)
        .expect("actual shared class, continuations and selected/replayed consumers");
}
fn uncertified_resident_identity_keeps_equal_source_bytes_in_distinct_populations() {
    exercise(Residency::Unstable).expect("real unstable source barrier");
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    SharedSourceContinuationFailureAndResultConsumersPreserveOneCertifiedPopulation,
    UncertifiedResidentIdentityKeepsEqualSourceBytesInDistinctPopulations,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "shared_source_continuation_failure_and_result_consumers_preserve_one_certified_population",
        ),
        word: Word::Succeeds,
        input: || {
            Scenario::SharedSourceContinuationFailureAndResultConsumersPreserveOneCertifiedPopulation
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "uncertified_resident_identity_keeps_equal_source_bytes_in_distinct_populations",
        ),
        word: Word::Succeeds,
        input: || Scenario::UncertifiedResidentIdentityKeepsEqualSourceBytesInDistinctPopulations,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::SharedSourceContinuationFailureAndResultConsumersPreserveOneCertifiedPopulation => {
            shared_source_continuation_failure_and_result_consumers_preserve_one_certified_population();
            Ok(())
        }
        Scenario::UncertifiedResidentIdentityKeepsEqualSourceBytesInDistinctPopulations => {
            uncertified_resident_identity_keeps_equal_source_bytes_in_distinct_populations();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
