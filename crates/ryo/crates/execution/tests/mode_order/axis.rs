//! Actual fresh streamed and held Axis admission.
use super::*;
#[test]
fn held_axis_empty_primary_skips_both_unread_sides_on_fresh_forward_route() {
    let mut input = authored_request(1);
    input.plans[0].steps = vec![
        step(select(Relation::Function)),
        step(axis(
            Input::Step(StepId(0)),
            Input::Relation(Relation::Defines),
            98,
        )),
    ];
    input.plans[0].frontier = vec![Input::Step(StepId(0))];
    input.plans[0].emit.step = StepId(1);
    let empty = Table {
        kinds: kinds(Relation::Function).to_vec(),
        count: RowCount(0),
        cells: vec![],
    };
    let side_reads = Cell::new(0);
    let read = |relation| {
        if relation == Relation::Function {
            Ok(SourceRows::Computed(&empty))
        } else {
            side_reads.set(side_reads.get() + 1);
            Err(OperatingError::NotRead)
        }
    };
    let forwarded = deliver(input, &read);
    assert!(forwarded.result.is_ok());
    assert!(forwarded.rows.is_empty());
    assert_eq!(side_reads.get(), 0);
}

#[test]
fn axis_admission_uses_side_first_only_for_the_exact_streamed_held_held_triple() {
    for domain_streamed in [false, true] {
        let mut input = authored_request(1);
        input.plans[0].steps = vec![step(select(Relation::Function))];
        let domain = if domain_streamed {
            input.plans[0].steps.push(step(select(Relation::Defines)));
            Input::Step(StepId(1))
        } else {
            Input::Relation(Relation::Defines)
        };
        input.plans[0]
            .steps
            .push(step(axis(Input::Step(StepId(0)), domain, 0)));
        input.plans[0].emit.step = StepId((input.plans[0].steps.len() - 1) as u32);
        let reads = std::cell::RefCell::new(vec![]);
        let read = |relation| {
            reads.borrow_mut().push(relation);
            Err::<SourceRows<'_>, _>(
                if (domain_streamed && relation == Relation::Function)
                    || (!domain_streamed && relation == Relation::SemanticChild)
                {
                    OperatingError::WrongBinding
                } else {
                    OperatingError::NotRead
                },
            )
        };
        let forwarded = deliver(input, &read);
        assert!(matches!(
            forwarded.result,
            Err(OperatingError::WrongBinding)
        ));
        assert!(forwarded.rows.is_empty());
        assert_eq!(
            reads.borrow().as_slice(),
            if domain_streamed {
                &[Relation::Function]
            } else {
                &[Relation::SemanticChild]
            }
        );
    }
}
