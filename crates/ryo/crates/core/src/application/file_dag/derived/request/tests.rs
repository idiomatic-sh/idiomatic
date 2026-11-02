use super::*;
use crate::domain::{
    derived_views::{collections::views::registration::registered, types::DerivedStorage},
    operator::contract::resources::{AxisResources, JoinResources},
};
use ryo_language::domain::relations::types::CellKind;

fn kinds() -> Vec<Vec<CellKind>> {
    vec![
        vec![CellKind::Number, CellKind::Number],
        vec![CellKind::Number, CellKind::Number],
        vec![CellKind::Number],
    ]
}

#[test]
fn missing_input_is_prepared_but_refused_only_when_the_view_is_demanded() {
    let mut views = RequestViews::default();
    let program = ResourceProgram::Index {
        input: 1,
        columns: IndexColumns::Whole,
    };
    let first = views.bind_mode(&program, &kinds()[..1]).unwrap();
    let use_plan = first[slot(DerivedKind::Equality)].as_ref().unwrap();
    assert_eq!(use_plan.inputs, [1, 0]);
    assert_eq!(use_plan.len, 1);
    assert_eq!(
        use_plan.plan.metadata,
        Err(OperatingError::MissingInput { position: 2 })
    );
    assert_eq!(
        registered(DerivedKind::Equality)
            .view(&use_plan.plan, &DerivedStorage::Unbuilt)
            .err(),
        Some(OperatingError::MissingInput { position: 2 }),
    );
    let second = views.bind_mode(&program, &kinds()[..1]).unwrap();
    assert!(Arc::ptr_eq(
        &use_plan.plan,
        &second[slot(DerivedKind::Equality)].as_ref().unwrap().plan
    ));
    assert_eq!(views.count(), 1);
}

#[test]
fn plain_join_and_axis_bind_finite_ordered_inputs_and_share_identical_plans() {
    let mut views = RequestViews::default();
    let kinds = kinds();
    let join = ResourceProgram::Join(JoinResources {
        output: vec![],
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(1)],
        view: vec![ColumnIndex(0)],
        has_filter: false,
        reach: None,
    });
    let first = views.bind_mode(&join, &kinds[..2]).unwrap();
    let second = views.bind_mode(&join, &kinds[..2]).unwrap();
    let equality = first[slot(DerivedKind::Equality)].as_ref().unwrap();
    let lookup = first[slot(DerivedKind::NativeLookup)].as_ref().unwrap();
    assert_eq!((equality.inputs, equality.len), ([1, 0], 1));
    assert_eq!((lookup.inputs, lookup.len), ([0, 1], 2));
    assert!(Arc::ptr_eq(
        &equality.plan,
        &second[slot(DerivedKind::Equality)].as_ref().unwrap().plan
    ));
    assert!(Arc::ptr_eq(
        &lookup.plan,
        &second[slot(DerivedKind::NativeLookup)]
            .as_ref()
            .unwrap()
            .plan
    ));
    assert_eq!(views.count(), 3);

    let axis = ResourceProgram::Axis(AxisResources {
        id: ColumnIndex(0),
        parent: ColumnIndex(0),
        child: ColumnIndex(1),
        domain_id: ColumnIndex(0),
        has_filter: false,
        domain_view: vec![],
        output: vec![],
        direction: Direction::Parent,
    });
    let slots = views.bind_mode(&axis, &kinds).unwrap();
    let edge = slots[slot(DerivedKind::Equality)].as_ref().unwrap();
    let ViewRequest::Equality { columns } = &edge.plan.request else {
        panic!("edge lookup")
    };
    assert_eq!(columns, &[ColumnIndex(1)]);
    assert_eq!(edge.inputs, [1, 0]);
    assert_eq!(
        slots[slot(DerivedKind::Membership)]
            .as_ref()
            .unwrap()
            .inputs,
        [2, 1]
    );
}

#[test]
fn family_slots_hold_compact_shared_binding_handles() {
    assert_eq!(size_of::<DerivedSlots>(), 24 * size_of::<usize>());
    let mut views = RequestViews::default();
    let schema = kinds();
    let first = views
        .bind_mode(
            &ResourceProgram::Index {
                input: 1,
                columns: IndexColumns::Columns(vec![ColumnIndex(0)]),
            },
            &schema,
        )
        .unwrap();
    let second = views
        .bind_mode(
            &ResourceProgram::Index {
                input: 1,
                columns: IndexColumns::Columns(vec![ColumnIndex(0)]),
            },
            &schema,
        )
        .unwrap();
    assert!(Arc::ptr_eq(
        &first[slot(DerivedKind::Equality)].as_ref().unwrap().primary,
        &second[slot(DerivedKind::Equality)]
            .as_ref()
            .unwrap()
            .primary
    ));
}
