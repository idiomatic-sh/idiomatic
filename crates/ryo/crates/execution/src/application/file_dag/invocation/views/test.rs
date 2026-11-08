//! The exact lease getter must consume the registered borrowed view without copying storage.
use super::*;
use ryo_derived::domain::contract::types::DerivedView;
use ryo_relations::domain::relations::types::{CellKind, Integer};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::sync::Arc;
use {
    ryo_execution_domain::domain::derived_views::types::*, ryo_execution_domain::domain::operation_schema::types::InputShape,
    ryo_values::domain::rows::service::push_row,
};
#[test]
fn production_cycle_lease_borrows_actual_trait_output_and_refuses_foreign_storage() {
    let mut table = Table {
        kinds: vec![CellKind::Number; 2],
        ..Default::default()
    };
    push_row(
        &mut table,
        &[Value::Number(Integer(1)), Value::Number(Integer(2))],
    );
    let plan = Arc::new(
        ryo_execution_store::application::file_dag::derived::prepare::view(
            (std::sync::Arc::new(
                ryo_execution_store::application::file_dag::fixtures::derived(),
            ) as std::sync::Arc<dyn ryo_execution_store::outbound::derived::interface::DerivedPort>),
            &ViewRequest::CycleAdjacency {
                source: ColumnIndex(0),
                destination: ColumnIndex(1),
            },
            &[InputShape {
                kinds: &table.kinds,
                tree: None,
            }],
        )
        .unwrap(),
    );
    let owner = ryo_execution_store::application::file_dag::relations::RelationOwner::new(
        ryo_execution_store::application::file_dag::relations::types::ReuseEligibility::Eligible,
        (std::sync::Arc::new(
            ryo_execution_store::application::file_dag::fixtures::derived(),
        ) as std::sync::Arc<dyn ryo_execution_store::outbound::derived::interface::DerivedPort>),
    );
    let publication = owner.computed(table.clone()).unwrap();
    let held = owner
        .resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))
        .unwrap();
    let rows = RepresentationInput::from(&held);
    let proof = owner.admit(publication.as_ref(), rows).unwrap();
    let acquired = owner
        .derived()
        .acquire_stored(
            &plan,
            &owner.derived().admit_view(&plan, &owner, &[&proof]).unwrap(),
            ryo_execution_store::application::file_dag::derived::owner::ViewContext::default(),
            DerivedInputs::Rows(&[rows]),
            &mut DerivedScratch::default(),
            None,
        )
        .unwrap();
    let DerivedView::CycleAdjacency(built) = acquired.view().unwrap() else {
        panic!("actual cycle builder");
    };
    let pointer = built.values.as_ptr();
    let graph = InvocationResources::cycles_view(Some(&acquired))
        .unwrap()
        .unwrap();
    assert_eq!(
        graph.values,
        [Value::Number(Integer(1)), Value::Number(Integer(2))]
    );
    assert_eq!(
        pointer,
        graph.values.as_ptr(),
        "the semantic lease borrows the owned graph"
    );
    let equality = Arc::new(
        ryo_execution_store::application::file_dag::derived::prepare::view(
            (std::sync::Arc::new(
                ryo_execution_store::application::file_dag::fixtures::derived(),
            ) as std::sync::Arc<dyn ryo_execution_store::outbound::derived::interface::DerivedPort>),
            &ViewRequest::Equality {
                columns: vec![ColumnIndex(0)],
            },
            &[InputShape {
                kinds: &table.kinds,
                tree: None,
            }],
        )
        .unwrap(),
    );
    let wrong = owner
        .derived()
        .acquire_stored(
            &equality,
            &owner
                .derived()
                .admit_view(&equality, &owner, &[&proof])
                .unwrap(),
            ryo_execution_store::application::file_dag::derived::owner::ViewContext::default(),
            DerivedInputs::Rows(&[rows]),
            &mut DerivedScratch::default(),
            None,
        )
        .unwrap();
    assert!(matches!(
        InvocationResources::cycles_view(Some(&wrong)),
        Err(OperatingError::WrongOperation)
    ));
}

use ryo_values::domain::rows::types::Table;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
