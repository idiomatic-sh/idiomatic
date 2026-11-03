//! The exact lease getter must consume the registered borrowed view without copying storage.
use super::*;
use crate::domain::{
    derived_views::{collections::views::registration::registered, types::*},
    operation_schema::types::InputSchema,
    table_model::service::push_row,
};
use ryo_relations::domain::relations::types::{CellKind, Integer};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::sync::Arc;
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
        registered(DerivedKind::CycleAdjacency)
            .prepare(
                &ViewRequest::CycleAdjacency {
                    source: ColumnIndex(0),
                    destination: ColumnIndex(1),
                },
                &[InputSchema {
                    kinds: &table.kinds,
                    tree: None,
                }],
            )
            .unwrap(),
    );
    let owner = crate::application::file_dag::relations::RelationOwner::new(
        false,
        crate::outbound::derived::factory::test_port(),
    );
    let rows = BorrowedRows::Computed(&table);
    let proof = owner.admit_external(rows).unwrap();
    let (population, _) = proof.identity(&owner).unwrap();
    let acquired = owner
        .derived
        .acquire_stored(
            &plan,
            &[population],
            crate::application::file_dag::derived::owner::ViewContext::default(),
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
        registered(DerivedKind::Equality)
            .prepare(
                &ViewRequest::Equality {
                    columns: vec![ColumnIndex(0)],
                },
                &[InputSchema {
                    kinds: &table.kinds,
                    tree: None,
                }],
            )
            .unwrap(),
    );
    let wrong = owner
        .derived
        .acquire_stored(
            &equality,
            &[population],
            crate::application::file_dag::derived::owner::ViewContext::default(),
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

use ryo_derived::domain::contract::types::DerivedKind;
