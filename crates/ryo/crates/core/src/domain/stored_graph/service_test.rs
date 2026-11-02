//! Compact adjacency matches ordinary closure without enumerating unrelated roots.

use super::service::build;
use super::types::traversal::value::GraphTraversal;
use super::types::{GraphIdentity, GraphWalk, Orientation, Population};
use crate::application::file_dag::testing::TestOperator as _;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::collections::operators::closure::item::ClosureItem;
use crate::domain::table_model::service::{push_row, row_in};
use crate::domain::table_model::types::{Rows, Table};
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::relations::types::{CellKind, Integer, Relation, Value};
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input};
use ryo_values::domain::plans::types::Operation;

/// Numeric synthetic graph rows, independent of any frontend.
fn table(rows: &[&[i64]], width: usize) -> Table {
    let mut out = Table {
        kinds: vec![CellKind::Number; width],
        ..Table::default()
    };
    for row in rows {
        push_row(
            &mut out,
            &row.iter()
                .map(|value| Value::Number(Integer(*value)))
                .collect::<Vec<_>>(),
        );
    }
    out
}

/// Identity need not correspond to the schema of the named frontend relation in a standalone graph test.
fn identity(keys: Vec<ColumnIndex>, rows: usize) -> GraphIdentity {
    GraphIdentity {
        population: Population {
            relation: Relation::Function,
            kinds: &[CellKind::Number, CellKind::Number, CellKind::Number],
            rows,
            native_file: None,
            selected: None,
            positions: None,
        },
        endpoints: [ColumnIndex(0), ColumnIndex(1)],
        keys,
    }
}

#[test]
fn compact_positive_reach_matches_sorted_closure_with_partitions_cycles_and_duplicates() {
    let edges = table(
        &[
            &[9, 2, 1],
            &[2, 3, 1],
            &[9, 2, 1],
            &[3, 9, 1],
            &[2, 2, 1],
            &[9, 8, 2],
            &[8, 7, 2],
            &[40, 41, 3],
        ],
        3,
    );
    let graph = build(
        identity(vec![ColumnIndex(2)], edges.count.0),
        &Rows::Computed(&edges),
        [ColumnIndex(0), ColumnIndex(1)],
        &[ColumnIndex(2)],
    )
    .expect("columns")
    .expect("graph");
    let mut symbols = Symbols::default();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        stored_indexes: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut Default::default(),
    };
    let mut ordinary = Table::default();
    ClosureItem
        .compute(
            &Operation::Closure {
                edges: Input::Relation(Relation::Function),
                source: ColumnIndex(0),
                destination: ColumnIndex(1),
                keys: vec![ColumnIndex(2)],
            },
            &[Rows::Computed(&edges)],
            &mut context,
            &mut ordinary,
        )
        .expect("ordinary closure");
    let mut walk = GraphWalk::default();
    for (root, value) in graph.vertices.iter().enumerate() {
        let partition = &graph.partitions[graph.vertex_partitions[root] as usize];
        let mut traversal = GraphTraversal::new(&graph, &mut walk);
        let _ = traversal.breadth(root as u32, Orientation::Forward, None);
        traversal.sort();
        let actual: Vec<_> = walk
            .reached
            .iter()
            .map(|vertex| graph.vertices[*vertex as usize])
            .collect();
        let expected: Vec<_> = (0..ordinary.count.0)
            .filter_map(|row| row_in(&ordinary, crate::domain::table_cells::types::RowIndex(row)))
            .filter(|row| row[0] == partition[0] && row[1] == *value)
            .map(|row| row[2])
            .collect();
        assert_eq!(actual, expected, "root {value:?} partition {partition:?}");
    }
}

#[test]
fn sparse_demand_work_is_independent_of_unrelated_components() {
    let mut edges = table(&[&[1, 2], &[2, 3]], 2);
    for index in 0..300 {
        push_row(
            &mut edges,
            &[
                Value::Number(Integer(1000 + index * 2)),
                Value::Number(Integer(1001 + index * 2)),
            ],
        );
    }
    let graph = build(
        identity(vec![], edges.count.0),
        &Rows::Computed(&edges),
        [ColumnIndex(0), ColumnIndex(1)],
        &[],
    )
    .expect("columns")
    .expect("graph");
    let root = graph.vertex_ids[&(0, Value::Number(Integer(1)))];
    let mut walk = GraphWalk::default();
    let _ = GraphTraversal::new(&graph, &mut walk).breadth(root, Orientation::Forward, None);
    assert_eq!(
        (walk.expanded, walk.examined, walk.reached.len()),
        (3, 2, 2)
    );
    // Repeated requests reuse epoch memory, not a global O(vertices) clear.
    let epoch = walk.epoch;
    let _ = GraphTraversal::new(&graph, &mut walk).breadth(root, Orientation::Forward, None);
    assert_eq!(walk.epoch, epoch + 1);
    assert_eq!((walk.expanded, walk.examined), (3, 2));
}

#[test]
fn invalid_columns_are_rejected_even_on_empty_edges() {
    let edges = table(&[], 2);
    assert!(
        build(
            identity(vec![], 0),
            &Rows::Computed(&edges),
            [ColumnIndex(0), ColumnIndex(2)],
            &[]
        )
        .is_err()
    );
    assert!(
        build(
            identity(vec![ColumnIndex(3)], 0),
            &Rows::Computed(&edges),
            [ColumnIndex(0), ColumnIndex(1)],
            &[ColumnIndex(3)]
        )
        .is_err()
    );
}

#[test]
fn typed_endpoint_identity_never_coalesces_equal_payload_words() {
    let mut edges = Table {
        kinds: vec![CellKind::Number; 2],
        ..Default::default()
    };
    let number = Value::Number(Integer(0));
    let flag = Value::Flag(ryo_language::domain::relations::types::Flag::Lowered);
    push_row(&mut edges, &[number, flag]);
    push_row(&mut edges, &[flag, number]);
    let graph = build(
        identity(vec![], 2),
        &Rows::Computed(&edges),
        [ColumnIndex(0), ColumnIndex(1)],
        &[],
    )
    .expect("columns")
    .expect("graph");
    assert_eq!(graph.vertices.len(), 2);
    assert_ne!(graph.vertex_ids[&(0, number)], graph.vertex_ids[&(0, flag)]);
}
