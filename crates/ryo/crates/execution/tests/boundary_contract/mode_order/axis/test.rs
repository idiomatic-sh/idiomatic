//! Delivery never selects a different Axis dependency contract.
use crate::mode_order::fixtures::*;
use crate::*;

#[test]
fn empty_axis_has_identical_admission_for_both_delivery_apis() {
    let authored = || {
        let mut input = authored_request(1);
        crate::boundary_fixture::fixtures::plans(&mut input)[0].steps = vec![
            step(select(Relation::Function)),
            step(axis(
                Input::Step(StepId(0)),
                Input::Relation(Relation::Defines),
                98,
            )),
        ];
        crate::boundary_fixture::fixtures::plans(&mut input)[0].frontier =
            vec![Input::Step(StepId(0))];
        crate::boundary_fixture::fixtures::plans(&mut input)[0]
            .emit
            .step = StepId(1);
        input
    };
    let empty = Table {
        kinds: kinds(Relation::Function).to_vec(),
        count: RowCount(0),
        cells: vec![],
    };
    let reads = std::cell::RefCell::new(vec![]);
    let read = |relation| {
        reads.borrow_mut().push(relation);
        if relation == Relation::Function {
            Ok(SourceRows::Computed(&empty))
        } else {
            Err(OperatingError::NotRead)
        }
    };
    let collected = evaluate(authored(), &read);
    let collected_reads = reads.take();
    let callback = deliver(authored(), &read);
    assert_eq!(
        *reads.borrow(),
        collected_reads,
        "delivery must not change demanded sources"
    );
    assert!(matches!(
        collected,
        RootOutcome::Failed(OperatingError::NotRead)
    ));
    assert!(matches!(callback.result, Err(OperatingError::NotRead)));
    assert!(
        collected_reads.contains(&Relation::SemanticChild),
        "empty primary cannot suppress the required edge input"
    );
    assert!(callback.rows.is_empty());
}

#[test]
fn axis_error_order_is_identical_for_both_delivery_apis_and_source_shapes() {
    for domain_step in [false, true] {
        let authored = || {
            let mut input = authored_request(1);
            crate::boundary_fixture::fixtures::plans(&mut input)[0].steps =
                vec![step(select(Relation::Function))];
            let domain = if domain_step {
                crate::boundary_fixture::fixtures::plans(&mut input)[0]
                    .steps
                    .push(step(select(Relation::Defines)));
                Input::Step(StepId(1))
            } else {
                Input::Relation(Relation::Defines)
            };
            crate::boundary_fixture::fixtures::plans(&mut input)[0]
                .steps
                .push(step(axis(Input::Step(StepId(0)), domain, 0)));
            crate::boundary_fixture::fixtures::plans(&mut input)[0]
                .emit
                .step = StepId(
                u32::try_from(
                    crate::boundary_fixture::fixtures::plans(&mut input)[0]
                        .steps
                        .len()
                        - 1,
                )
                .expect("a step number fits in u32"),
            );
            input
        };
        let reads = std::cell::RefCell::new(vec![]);
        let read = |relation| {
            reads.borrow_mut().push(relation);
            Err::<SourceRows<'_>, _>(if relation == Relation::Function {
                OperatingError::WrongBinding
            } else {
                OperatingError::NotRead
            })
        };
        let RootOutcome::Failed(error) = evaluate(authored(), &read) else {
            panic!("the unavailable source must fail evaluation");
        };
        assert_eq!(error, OperatingError::WrongBinding);
        let collected_reads = reads.take();
        assert_eq!(collected_reads, vec![Relation::Function]);
        let callback = deliver(authored(), &read);
        assert!(matches!(callback.result, Err(actual) if actual == error));
        assert_eq!(*reads.borrow(), collected_reads);
        assert!(callback.rows.is_empty());
    }
}
