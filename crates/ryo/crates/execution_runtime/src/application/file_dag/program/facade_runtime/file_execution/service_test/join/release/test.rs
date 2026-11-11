//! Delivery refusal and a semantic refusal release every strategy; the semantic error is primary.
//! `execute` finishes the owner and asserts the shared ledger, working and owned bytes are zero.
use super::super::capability::fixtures::Witness;
use super::super::*;
use super::fixtures::{OrderedPort, refused};
use super::fixtures::{cancelled, ordered, tree_join};
use crate::cases::{Case, CaseName, Expect, Word, run};
#[cfg(test)]
use fixtures::execution::*;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_values::domain::rows::types::RowCount;
use std::sync::Arc;
use std::sync::atomic::Ordering;

fn native_tree_join_delivery_refusal_releases_and_the_completed_join_delivers() {
    let witness = Arc::new(Witness::default());
    let cancelled = tree_join(witness.clone(), cancelled());
    assert!(witness.calls.load(Ordering::Relaxed) > 0);
    assert!(matches!(cancelled.result, Err(OperatingError::NotRun)));
    assert!(cancelled.prefix.is_empty());
    let complete = tree_join(Arc::new(Witness::default()), Faults::default());
    assert_eq!(complete.result.unwrap().count, RowCount(2));
    assert_eq!(complete.prefix.len(), 2);
}

fn native_tree_join_semantic_refusal_is_primary_over_the_sink_refusal() {
    let witness = Arc::new(Witness {
        refuse: true,
        ..Witness::default()
    });
    let actual = tree_join(witness.clone(), cancelled());
    assert!(witness.calls.load(Ordering::Relaxed) > 0);
    assert_eq!(actual.result.unwrap_err(), refused());
    assert!(actual.prefix.is_empty(), "the sink is never reached");
}

fn ordered_strategy_delivery_refusal_releases_and_the_completed_root_delivers() {
    let port = Arc::new(OrderedPort::default());
    let actual = ordered(port.clone(), cancelled()).unwrap();
    assert!(
        port.calls.load(Ordering::Relaxed) > 0,
        "the ordered-existence meaning ran before the sink refused"
    );
    assert!(matches!(actual.result, Err(OperatingError::NotRun)));
    assert!(actual.prefix.is_empty());
    let complete = ordered(Arc::new(OrderedPort::default()), Faults::default()).unwrap();
    assert_eq!(complete.result.unwrap().count, RowCount(1));
}

fn ordered_strategy_semantic_refusal_is_primary_over_the_sink_refusal() {
    let port = Arc::new(OrderedPort {
        refuse: true,
        ..OrderedPort::default()
    });
    let actual = ordered(port.clone(), cancelled()).unwrap();
    assert!(port.calls.load(Ordering::Relaxed) > 0);
    assert_eq!(actual.result.unwrap_err(), refused());
    assert!(actual.prefix.is_empty(), "the sink is never reached");
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    NativeTreeJoinDeliveryRefusalReleasesAndTheCompletedJoinDelivers,
    NativeTreeJoinSemanticRefusalIsPrimaryOverTheSinkRefusal,
    OrderedStrategyDeliveryRefusalReleasesAndTheCompletedRootDelivers,
    OrderedStrategySemanticRefusalIsPrimaryOverTheSinkRefusal,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "native_tree_join_delivery_refusal_releases_and_the_completed_join_delivers",
        ),
        word: Word::Succeeds,
        input: || Scenario::NativeTreeJoinDeliveryRefusalReleasesAndTheCompletedJoinDelivers,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("native_tree_join_semantic_refusal_is_primary_over_the_sink_refusal"),
        word: Word::Succeeds,
        input: || Scenario::NativeTreeJoinSemanticRefusalIsPrimaryOverTheSinkRefusal,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "ordered_strategy_delivery_refusal_releases_and_the_completed_root_delivers",
        ),
        word: Word::Succeeds,
        input: || Scenario::OrderedStrategyDeliveryRefusalReleasesAndTheCompletedRootDelivers,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("ordered_strategy_semantic_refusal_is_primary_over_the_sink_refusal"),
        word: Word::Succeeds,
        input: || Scenario::OrderedStrategySemanticRefusalIsPrimaryOverTheSinkRefusal,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::NativeTreeJoinDeliveryRefusalReleasesAndTheCompletedJoinDelivers => {
            native_tree_join_delivery_refusal_releases_and_the_completed_join_delivers();
            Ok(())
        }
        Scenario::NativeTreeJoinSemanticRefusalIsPrimaryOverTheSinkRefusal => {
            native_tree_join_semantic_refusal_is_primary_over_the_sink_refusal();
            Ok(())
        }
        Scenario::OrderedStrategyDeliveryRefusalReleasesAndTheCompletedRootDelivers => {
            ordered_strategy_delivery_refusal_releases_and_the_completed_root_delivers();
            Ok(())
        }
        Scenario::OrderedStrategySemanticRefusalIsPrimaryOverTheSinkRefusal => {
            ordered_strategy_semantic_refusal_is_primary_over_the_sink_refusal();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
