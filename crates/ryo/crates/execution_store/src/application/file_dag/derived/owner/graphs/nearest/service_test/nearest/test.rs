//! Nearest-ancestor answers through the shared derived forest, with exact domain and generation identity.

use crate::application::file_dag::derived::owner::graphs::fixtures::{ENDPOINTS, SOURCE, rows};
use crate::application::file_dag::derived::owner::{
    fixtures::graph::GraphFixture, index::service::LIMIT,
};
use ryo_execution_domain::domain::stored_graph::types::{GraphWalk, Orientation};
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::{CellKind, FactColumn, Integer, Relation, Value};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input};
use std::sync::Arc;

#[test]
fn nearest_memo_is_shared_only_for_exact_domain_and_generation() {
    let columns = [FactColumn(vec![1, 2, 8]), FactColumn(vec![2, 3, 2])];
    let rows = rows(&columns);
    let fixture = GraphFixture::new(&[Relation::Function, Relation::Identifier], &[], LIMIT);
    let graph = fixture
        .acquire(SOURCE, &rows, ENDPOINTS, &[])
        .unwrap()
        .unwrap();
    let domain_columns = [FactColumn(vec![3, 3]), FactColumn(vec![0, 0])];
    let domain_rows = self::rows(&domain_columns);
    let membership = fixture
        .domain(
            &graph,
            Input::Relation(Relation::Identifier),
            &domain_rows,
            ColumnIndex(0),
        )
        .unwrap()
        .unwrap();
    let again = fixture
        .domain(
            &graph,
            Input::Relation(Relation::Identifier),
            &domain_rows,
            ColumnIndex(0),
        )
        .unwrap()
        .unwrap();
    assert!(Arc::ptr_eq(membership.raw_domain(), again.raw_domain()));
    assert_eq!(
        fixture.snapshot().domains,
        1,
        "exact membership shares one domain"
    );
    let root = graph
        .graph()
        .vertex(
            ryo_derived::domain::vertices::types::PartitionId(0),
            Value::Number(Integer(1)),
        )
        .expect("stored vertex")
        .0;
    let mut walk = GraphWalk::default();
    let mut local = Vec::new();
    let first = fixture
        .nearest(
            &graph,
            &membership,
            root,
            Orientation::Forward,
            &mut local,
            &mut walk,
        )
        .unwrap();
    assert_eq!(
        first.map(|vertex| graph.graph().vertices()
            [usize::try_from(vertex).expect("a vertex id fits in usize")]),
        Some(Value::Number(Integer(3)))
    );
    assert_eq!(fixture.snapshot().nearest, 1);
    assert_eq!(
        fixture
            .nearest(
                &graph,
                &again,
                root,
                Orientation::Forward,
                &mut local,
                &mut walk
            )
            .unwrap(),
        first
    );
    assert_eq!(fixture.snapshot().counts.forest_hits, 1);
    // A computed domain cannot borrow stable relation membership identity.
    let table = ryo_values::domain::rows::types::Table {
        kinds: vec![CellKind::Number],
        cells: vec![Value::Number(Integer(2))],
        count: ryo_values::domain::rows::types::RowCount(1),
    };
    let different = fixture
        .domain(
            &graph,
            Input::Relation(Relation::Identifier),
            &Rows::Computed(&table),
            ColumnIndex(0),
        )
        .unwrap()
        .unwrap();
    local.clear();
    let changed = fixture
        .nearest(
            &graph,
            &different,
            root,
            Orientation::Forward,
            &mut local,
            &mut walk,
        )
        .unwrap();
    assert_eq!(
        changed.map(|vertex| graph.graph().vertices()
            [usize::try_from(vertex).expect("a vertex id fits in usize")]),
        Some(Value::Number(Integer(2)))
    );
    assert_eq!(fixture.snapshot().nearest, 1);
    assert!(fixture.snapshot().memo_bytes <= super::super::super::super::service::MEMO_LIMIT);
    assert!(fixture.snapshot().bytes <= LIMIT);
}

#[test]
fn nearest_handles_absent_parent_and_normal_parent_through_derived_forest() {
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![2, 3])];
    let rows = rows(&columns);
    let fixture = GraphFixture::new(&[Relation::Function, Relation::Identifier], &[], LIMIT);
    let graph = fixture
        .acquire(SOURCE, &rows, ENDPOINTS, &[])
        .unwrap()
        .unwrap();
    let domain_columns = [FactColumn(vec![2]), FactColumn(vec![0])];
    let domain_rows = self::rows(&domain_columns);
    let membership = fixture
        .domain(
            &graph,
            Input::Relation(Relation::Identifier),
            &domain_rows,
            ColumnIndex(0),
        )
        .unwrap()
        .unwrap();
    let mut walk = GraphWalk::default();
    let mut local = Vec::new();
    for (root, expected) in [(3, None), (1, Some(2))] {
        let root = graph
            .graph()
            .vertex(
                ryo_derived::domain::vertices::types::PartitionId(0),
                Value::Number(Integer(root)),
            )
            .unwrap()
            .0;
        let answer = fixture
            .nearest(
                &graph,
                &membership,
                root,
                Orientation::Forward,
                &mut local,
                &mut walk,
            )
            .unwrap();
        assert_eq!(
            answer.map(|id| graph.graph().vertices()
                [usize::try_from(id).expect("a vertex id fits in usize")]),
            expected.map(|n| Value::Number(Integer(n)))
        );
    }
}
