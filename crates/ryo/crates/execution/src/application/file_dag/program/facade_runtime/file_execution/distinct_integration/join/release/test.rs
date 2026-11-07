//! Delivery refusal and a semantic refusal release every strategy; the semantic error is primary.
//! `execute` finishes the owner and asserts the shared ledger, working and owned bytes are zero.
use super::super::capability::fixtures::Witness;
use super::super::cases::test::{node, table_of, tree_rows, wide_tree};
use super::super::column_chain::fixtures::{expected, plan, sources};
use super::super::*;
use super::fixtures::{OrderedPort, ordered_sources, refused};
use std::sync::atomic::Ordering;

fn cancelled() -> Faults {
    Faults {
        sink_after: Some(0),
        ..Faults::default()
    }
}

fn tree_join(
    port: Arc<dyn crate::outbound::operators::interface::OperatorsPort>,
    faults: Faults,
) -> Observation {
    let (elements, mut symbols) = wide_tree(1, false);
    let left = table_of(&[CellKind::Node], &[&[node(1)], &[node(2)]]);
    let join = operation(
        &[0],
        &[1],
        &[(Side::Right, 0), (Side::Right, 1)],
        Vec::new(),
        &[],
    );
    invoke(
        &join,
        [Rows::Computed(&left), tree_rows(&elements, TreeForm::Edges)],
        &mut symbols,
        b"abcd",
        true,
        port,
        faults,
    )
    .unwrap()
}

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

fn ordered(port: Arc<OrderedPort>, faults: Faults) -> Result<Observation, OperatingError> {
    let (request, _) = super::super::super::route_controls::test::ordered_strategy_fixture();
    execute(
        request,
        ordered_sources(),
        &mut Symbols::default(),
        b"",
        true,
        port,
        faults,
    )
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
