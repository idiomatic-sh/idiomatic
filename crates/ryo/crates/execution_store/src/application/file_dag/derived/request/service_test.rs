use super::types::PlanCount;
use super::*;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operator::contract::resources::{AxisResources, JoinResources};
use ryo_relations::domain::relations::types::CellKind;

fn kinds() -> Vec<Vec<CellKind>> {
    vec![
        vec![CellKind::Number, CellKind::Number],
        vec![CellKind::Number, CellKind::Number],
        vec![CellKind::Number],
    ]
}

fn missing_input_is_prepared_but_refused_only_when_the_view_is_demanded() {
    let mut views = RequestViews::new(std::sync::Arc::new(
        crate::application::file_dag::fixtures::derived(),
    ));
    let program = ResourceProgram::Index {
        input: ryo_values::domain::plan_parts::types::InputNumber(1),
        columns: IndexColumns::Whole,
    };
    let first = views.bind_resources(&program, &kinds()[..1]).unwrap();
    let use_plan = &first[DerivedKind::Equality.ordinal()]
        .as_ref()
        .unwrap()
        .primary;
    assert_eq!(use_plan.inputs, [InputNumber(1), InputNumber(0)]);
    assert_eq!(use_plan.len, InputsUsed(1));
    assert_eq!(
        use_plan.plan.metadata,
        Err(OperatingError::MissingInput { position: 2 })
    );
    assert_eq!(
        ryo_execution_domain::domain::derived_views::service::validate(
            DerivedKind::Equality,
            &use_plan.plan
        )
        .err(),
        Some(OperatingError::MissingInput { position: 2 }),
    );
    let second = views.bind_resources(&program, &kinds()[..1]).unwrap();
    assert!(Arc::ptr_eq(
        &use_plan.plan,
        &second[DerivedKind::Equality.ordinal()]
            .as_ref()
            .unwrap()
            .primary
            .plan
    ));
    assert_eq!(views.count(), PlanCount(1));
}

fn plain_join_and_axis_bind_finite_ordered_inputs_and_share_identical_plans() {
    let mut views = RequestViews::new(std::sync::Arc::new(
        crate::application::file_dag::fixtures::derived(),
    ));
    let kinds = kinds();
    let join = ResourceProgram::Join(JoinResources {
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(1)],
        view: vec![ColumnIndex(0)],
        has_filter:
            ryo_execution_domain::domain::operator::contract::resources::Filtering::Unfiltered,
        reach: None,
    });
    let first = views.bind_resources(&join, &kinds[..2]).unwrap();
    let second = views.bind_resources(&join, &kinds[..2]).unwrap();
    let equality = &first[DerivedKind::Equality.ordinal()]
        .as_ref()
        .unwrap()
        .primary;
    let lookup = &first[DerivedKind::NativeLookup.ordinal()]
        .as_ref()
        .unwrap()
        .primary;
    assert_eq!(
        (equality.inputs, equality.len),
        ([InputNumber(1), InputNumber(0)], InputsUsed(1))
    );
    assert_eq!(
        (lookup.inputs, lookup.len),
        ([InputNumber(0), InputNumber(1)], InputsUsed(2))
    );
    assert!(Arc::ptr_eq(
        &equality.plan,
        &second[DerivedKind::Equality.ordinal()]
            .as_ref()
            .unwrap()
            .primary
            .plan
    ));
    assert!(Arc::ptr_eq(
        &lookup.plan,
        &second[DerivedKind::NativeLookup.ordinal()]
            .as_ref()
            .unwrap()
            .primary
            .plan
    ));
    assert_eq!(views.count(), PlanCount(3));

    let axis = ResourceProgram::Axis(AxisResources {
        id: ColumnIndex(0),
        parent: ColumnIndex(0),
        child: ColumnIndex(1),
        domain_id: ColumnIndex(0),
        has_filter:
            ryo_execution_domain::domain::operator::contract::resources::Filtering::Unfiltered,
        domain_view: vec![],
        output: vec![],
        direction: Direction::Parent,
    });
    let slots = views.bind_resources(&axis, &kinds).unwrap();
    let edge = &slots[DerivedKind::Equality.ordinal()]
        .as_ref()
        .unwrap()
        .primary;
    let ViewRequest::Equality { columns } = &edge.plan.request else {
        panic!("edge lookup")
    };
    assert_eq!(columns, &[ColumnIndex(1)]);
    assert_eq!(edge.inputs, [InputNumber(1), InputNumber(0)]);
    assert_eq!(
        slots[DerivedKind::Membership.ordinal()]
            .as_ref()
            .unwrap()
            .primary
            .inputs,
        [InputNumber(2), InputNumber(1)]
    );
}

fn family_slots_hold_compact_shared_binding_handles() {
    assert_eq!(
        size_of::<DerivedSlots>(),
        DerivedKind::ALL.len() * size_of::<usize>()
    );
    let mut views = RequestViews::new(std::sync::Arc::new(
        crate::application::file_dag::fixtures::derived(),
    ));
    let schema = kinds();
    let first = views
        .bind_resources(
            &ResourceProgram::Index {
                input: ryo_values::domain::plan_parts::types::InputNumber(1),
                columns: IndexColumns::Columns(vec![ColumnIndex(0)]),
            },
            &schema,
        )
        .unwrap();
    let second = views
        .bind_resources(
            &ResourceProgram::Index {
                input: ryo_values::domain::plan_parts::types::InputNumber(1),
                columns: IndexColumns::Columns(vec![ColumnIndex(0)]),
            },
            &schema,
        )
        .unwrap();
    assert!(Arc::ptr_eq(
        &first[DerivedKind::Equality.ordinal()]
            .as_ref()
            .unwrap()
            .primary,
        &second[DerivedKind::Equality.ordinal()]
            .as_ref()
            .unwrap()
            .primary
    ));
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    MissingInputIsPreparedButRefusedOnlyWhenTheViewIsDemanded,
    PlainJoinAndAxisBindFiniteOrderedInputsAndShareIdenticalPlans,
    FamilySlotsHoldCompactSharedBindingHandles,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("missing_input_is_prepared_but_refused_only_when_the_view_is_demanded"),
        word: Word::Succeeds,
        input: || Scenario::MissingInputIsPreparedButRefusedOnlyWhenTheViewIsDemanded,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("plain_join_and_axis_bind_finite_ordered_inputs_and_share_identical_plans"),
        word: Word::Succeeds,
        input: || Scenario::PlainJoinAndAxisBindFiniteOrderedInputsAndShareIdenticalPlans,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("family_slots_hold_compact_shared_binding_handles"),
        word: Word::Succeeds,
        input: || Scenario::FamilySlotsHoldCompactSharedBindingHandles,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::MissingInputIsPreparedButRefusedOnlyWhenTheViewIsDemanded => {
            missing_input_is_prepared_but_refused_only_when_the_view_is_demanded();
            Ok(())
        }
        Scenario::PlainJoinAndAxisBindFiniteOrderedInputsAndShareIdenticalPlans => {
            plain_join_and_axis_bind_finite_ordered_inputs_and_share_identical_plans();
            Ok(())
        }
        Scenario::FamilySlotsHoldCompactSharedBindingHandles => {
            family_slots_hold_compact_shared_binding_handles();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
