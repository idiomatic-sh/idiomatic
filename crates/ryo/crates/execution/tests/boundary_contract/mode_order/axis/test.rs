//! Delivery never selects a different Axis dependency contract.
use crate::mode_order::fixtures::*;
use crate::*;

#[test]
fn empty_axis_has_identical_admission_for_both_delivery_apis() {
    let authored = || {
        let mut input = authored_request(1);
        boundary_fixture::fixtures::plans(&mut input)[0].steps = vec![
            step(select(Relation::Function)),
            step(axis(
                Input::Step(StepId(0)),
                Input::Relation(Relation::Defines),
                98,
            )),
        ];
        boundary_fixture::fixtures::plans(&mut input)[0].frontier = vec![Input::Step(StepId(0))];
        boundary_fixture::fixtures::plans(&mut input)[0].emit.step = StepId(1);
        input
    };
    let empty = Table {
        kinds: kinds(Relation::Function).to_vec(),
        count: RowCount(0),
        cells: vec![],
    };
    let mut collected_facts = TableFacts::new(&[(Relation::Function, &empty)]);
    let collected = evaluate(authored(), &mut collected_facts);
    let collected_reads = collected_facts.history();
    assert!(matches!(collected, Err(FileStop::Facts(_))));
    assert!(
        collected_reads.contains(&Relation::SemanticChild),
        "empty primary cannot suppress the required edge input"
    );
}

#[test]
fn axis_error_order_is_identical_for_both_delivery_apis_and_source_shapes() {
    for domain_step in [false, true] {
        let authored = || {
            let mut input = authored_request(1);
            boundary_fixture::fixtures::plans(&mut input)[0].steps =
                vec![step(select(Relation::Function))];
            let domain = if domain_step {
                boundary_fixture::fixtures::plans(&mut input)[0]
                    .steps
                    .push(step(select(Relation::Defines)));
                Input::Step(StepId(1))
            } else {
                Input::Relation(Relation::Defines)
            };
            boundary_fixture::fixtures::plans(&mut input)[0]
                .steps
                .push(step(axis(Input::Step(StepId(0)), domain, 0)));
            boundary_fixture::fixtures::plans(&mut input)[0].emit.step = StepId(
                u32::try_from(boundary_fixture::fixtures::plans(&mut input)[0].steps.len() - 1)
                    .expect("a step number fits in u32"),
            );
            input
        };
        // No relation is held: the first read, of Function, is refused and stops the rule.
        let mut collected_facts = TableFacts::new(&[]);
        let Err(error) = evaluate(authored(), &mut collected_facts) else {
            panic!("the unavailable source must fail evaluation");
        };
        assert!(matches!(error, FileStop::Facts(_)));
        let collected_reads = collected_facts.history();
        assert_eq!(collected_reads, vec![Relation::Function]);
    }
}
