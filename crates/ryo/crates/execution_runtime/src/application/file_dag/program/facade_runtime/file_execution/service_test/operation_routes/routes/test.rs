//! Every production route publishes the independent expected rows.
use super::super::fixtures::chain::execute_fixture;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::query_operators::types::OperationKind;
use std::sync::Arc;
fn all26_actual_production_routes_publish_the_independent_expected_rows() {
    for &kind in OperationKind::ALL {
        // Each case uses the real normal scheduler for both an emitted root and
        // an authored downstream row consumer. Rows/Hosted remain request-owned sources.
        for downstream in [false, true] {
            execute_fixture(
                kind,
                downstream,
                Arc::new(crate::application::file_dag::fixtures::operators()),
            )
            .unwrap_or_else(|error| panic!("compiled {kind:?}: {error:?}"));
        }
    }
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    All26ActualProductionRoutesPublishTheIndependentExpectedRows,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("all26_actual_production_routes_publish_the_independent_expected_rows"),
    word: Word::Succeeds,
    input: || Scenario::All26ActualProductionRoutesPublishTheIndependentExpectedRows,
    expect: Expect::Value(()),
}];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::All26ActualProductionRoutesPublishTheIndependentExpectedRows => {
            all26_actual_production_routes_publish_the_independent_expected_rows();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
