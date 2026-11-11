//! Delivery refusal and a semantic refusal release every strategy; the semantic error is primary.
//! `execute` finishes the owner and asserts the shared ledger, working and owned bytes are zero.
use super::super::super::super::*;
use super::super::capability::fixtures::Witness;
use super::super::*;
use super::fixtures::{OrderedPort, refused};
use super::fixtures::{cancelled, ordered, tree_join};
use ryo_values::domain::rows::types::RowCount;
use std::sync::Arc;
use std::sync::atomic::Ordering;

#[test]
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

#[test]
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

#[test]
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

#[test]
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
