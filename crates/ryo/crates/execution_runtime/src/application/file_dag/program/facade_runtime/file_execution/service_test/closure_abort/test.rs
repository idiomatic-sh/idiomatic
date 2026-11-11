//! A real downstream refusal interrupts partition construction and releases its backing.
use super::fixtures::{Refusal, exercise};
use crate::cases::{Case, CaseName, Expect, Word, run};

fn later_partition_refusal_stops_work_and_releases_backing() {
    for refusal in [
        Refusal::CollectedPattern,
        Refusal::DeliveredPattern,
        Refusal::Callback,
    ] {
        for fused in [false, true] {
            exercise(refusal, fused);
        }
    }
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    LaterPartitionRefusalStopsWorkAndReleasesBacking,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("later_partition_refusal_stops_work_and_releases_backing"),
    word: Word::Succeeds,
    input: || Scenario::LaterPartitionRefusalStopsWorkAndReleasesBacking,
    expect: Expect::Value(()),
}];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::LaterPartitionRefusalStopsWorkAndReleasesBacking => {
            later_partition_refusal_stops_work_and_releases_backing();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
