use super::types::PlanCount;
use super::*;
use crate::domain::operator::contract::resources::{AxisResources, JoinResources};
use ryo_relations::domain::relations::types::CellKind;

fn kinds() -> Vec<Vec<CellKind>> {
    vec![
        vec![CellKind::Number, CellKind::Number],
        vec![CellKind::Number, CellKind::Number],
        vec![CellKind::Number],
    ]
}

#[test]
fn missing_input_is_prepared_but_refused_only_when_the_view_is_demanded() {
    let mut views = RequestViews::new(
        (std::sync::Arc::new(
            ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
    );
    let program = ResourceProgram::Index {
        input: ryo_values::domain::plan_parts::types::InputNumber(1),
        columns: IndexColumns::Whole,
    };
    let first = views.bind_resources(&program, &kinds()[..1]).unwrap();
    let use_plan = &first[DerivedKind::Equality.ordinal()].as_ref().unwrap().primary;
    assert_eq!(use_plan.inputs, [InputNumber(1), InputNumber(0)]);
    assert_eq!(use_plan.len, InputsUsed(1));
    assert_eq!(
        use_plan.plan.metadata,
        Err(OperatingError::MissingInput { position: 2 })
    );
    assert_eq!(
        crate::domain::derived_views::service::validate(DerivedKind::Equality, &use_plan.plan)
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

#[test]
fn plain_join_and_axis_bind_finite_ordered_inputs_and_share_identical_plans() {
    let mut views = RequestViews::new(
        (std::sync::Arc::new(
            ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
    );
    let kinds = kinds();
    let join = ResourceProgram::Join(JoinResources {
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(1)],
        view: vec![ColumnIndex(0)],
        has_filter: crate::domain::operator::contract::resources::Filtering::Unfiltered,
        reach: None,
    });
    let first = views.bind_resources(&join, &kinds[..2]).unwrap();
    let second = views.bind_resources(&join, &kinds[..2]).unwrap();
    let equality = &first[DerivedKind::Equality.ordinal()].as_ref().unwrap().primary;
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
        has_filter: crate::domain::operator::contract::resources::Filtering::Unfiltered,
        domain_view: vec![],
        output: vec![],
        direction: Direction::Parent,
    });
    let slots = views.bind_resources(&axis, &kinds).unwrap();
    let edge = &slots[DerivedKind::Equality.ordinal()].as_ref().unwrap().primary;
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

#[test]
fn family_slots_hold_compact_shared_binding_handles() {
    assert_eq!(
        size_of::<DerivedSlots>(),
        DerivedKind::ALL.len() * size_of::<usize>()
    );
    let mut views = RequestViews::new(
        (std::sync::Arc::new(
            ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
    );
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
        &first[DerivedKind::Equality.ordinal()].as_ref().unwrap().primary,
        &second[DerivedKind::Equality.ordinal()]
            .as_ref()
            .unwrap()
            .primary
    ));
}
