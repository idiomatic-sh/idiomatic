//! Per-row and request-owned routes beside a shared head: barrier, classification, isolation.
use super::super::fixtures::routes::RowRoute;
use super::fixtures::exercise;
use crate::cases::{Case, CaseName, Expect, Word, run};

fn per_row_and_request_owned_routes_are_barriers_beside_a_shared_head() {
    for route in RowRoute::ALL {
        exercise(route).unwrap_or_else(|error| panic!("{route:?}: {error:?}"));
    }
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    PerRowAndRequestOwnedRoutesAreBarriersBesideASharedHead,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("per_row_and_request_owned_routes_are_barriers_beside_a_shared_head"),
    word: Word::Succeeds,
    input: || Scenario::PerRowAndRequestOwnedRoutesAreBarriersBesideASharedHead,
    expect: Expect::Value(()),
}];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::PerRowAndRequestOwnedRoutesAreBarriersBesideASharedHead => {
            per_row_and_request_owned_routes_are_barriers_beside_a_shared_head();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
