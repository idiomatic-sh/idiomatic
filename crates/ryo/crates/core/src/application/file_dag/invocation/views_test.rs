//! The exact lease getter must consume the registered borrowed view without copying storage.
use super::*;
use crate::domain::{
    derived_views::{collections::views::registration::registered, types::*},
    operation_schema::types::InputSchema,
    table_model::{service::push_row, types::Table},
};
use ryo_language::domain::relations::types::{CellKind, Integer};
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
    let mut storage = DerivedStorage::Unbuilt;
    crate::application::file_dag::derived::owner::build_scoped(
        crate::application::file_dag::route_counts::BuildRoute::Fixture,
        &plan,
        DerivedInputs::Rows(&[BorrowedRows::Computed(&table)]),
        &mut storage,
        &mut DerivedScratch::default(),
    )
    .unwrap();
    let pointer = match &storage {
        DerivedStorage::CycleAdjacency(graph) => graph.values.as_ptr(),
        _ => panic!("actual cycle builder"),
    };
    let acquired = AcquiredView::new(&plan, Arc::new(storage));
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
    let wrong = AcquiredView::new(
        &plan,
        Arc::new(DerivedStorage::Equality(KeyIndex::default())),
    );
    assert!(matches!(
        InvocationResources::cycles_view(Some(&wrong)),
        Err(OperatingError::WrongOperation)
    ));
}
