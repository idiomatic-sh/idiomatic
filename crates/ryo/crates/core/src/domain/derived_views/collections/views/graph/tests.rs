//! Independent row-order and resource-shape controls for the graph view families.
use super::*;
use crate::domain::{
    operation_schema::types::InputSchema,
    stored_graph::types::{GraphIdentity, GraphSource, Orientation, Population},
    table_model::{service::push_row, types::Table},
};
use ryo_language::domain::relations::types::{CellKind, Integer, Relation, Value};
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::plan_parts::types::ColumnIndex;

fn number(value: i64) -> Value {
    Value::Number(Integer(value))
}
fn table(width: usize, rows: &[&[i64]]) -> Table {
    let mut table = Table {
        kinds: vec![CellKind::Number; width],
        ..Table::default()
    };
    for row in rows {
        push_row(
            &mut table,
            &row.iter().copied().map(number).collect::<Vec<_>>(),
        );
    }
    table
}
fn schema(width: usize) -> Vec<CellKind> {
    vec![CellKind::Number; width]
}

#[test]
fn partition_order_and_one_partition_closure_preserve_independent_expected_rows() {
    let source = table(3, &[&[9, 4, 3], &[9, 2, 4], &[1, 7, 8]]);
    let kinds = schema(3);
    let schemas = [InputSchema {
        kinds: &kinds,
        tree: None,
    }];
    let mut storage = DerivedStorage::Unbuilt;
    let mut scratch = DerivedScratch::default();
    let order = PartitionOrderItem;
    let contract = order
        .prepare(
            &ViewRequest::PartitionOrder {
                keys: vec![ColumnIndex(0)],
            },
            &schemas,
        )
        .unwrap();
    order
        .build(
            &contract,
            DerivedInputs::Rows(&[BorrowedRows::Computed(&source)]),
            &mut storage,
            &mut scratch,
        )
        .unwrap();
    let DerivedView::PartitionOrder(rows) = order.view(&contract, &storage).unwrap() else {
        panic!("partition order view")
    };
    assert_eq!(
        rows.iter().map(|row| row.0).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );

    // The caller supplies one current partition, rather than asking the item to retain all groups.
    let single = table(2, &[&[4, 3], &[2, 4], &[4, 3]]);
    let one_schema = [InputSchema {
        kinds: &[CellKind::Number, CellKind::Number],
        tree: None,
    }];
    let closure = ClosureAdjacencyItem;
    let contract = closure
        .prepare(
            &ViewRequest::ClosureAdjacency {
                source: ColumnIndex(0),
                destination: ColumnIndex(1),
            },
            &one_schema,
        )
        .unwrap();
    closure
        .build(
            &contract,
            DerivedInputs::Rows(&[BorrowedRows::Computed(&single)]),
            &mut storage,
            &mut scratch,
        )
        .unwrap();
    let DerivedView::ClosureAdjacency(part) = closure.view(&contract, &storage).unwrap() else {
        panic!("closure view")
    };
    assert!(part.key.is_empty());
    assert_eq!(
        part.table.cells,
        vec![
            number(4),
            number(3),
            number(2),
            number(4),
            number(4),
            number(3)
        ]
    );
    assert_eq!(part.sources, vec![number(2), number(4)]);
    assert_eq!(
        order.view(&contract, &storage).err(),
        Some(OperatingError::WrongOperation)
    );

    let mut malformed = table(2, &[&[4, 3]]);
    malformed.count.0 = 2;
    assert_eq!(
        closure.build(
            &contract,
            DerivedInputs::Rows(&[BorrowedRows::Computed(&malformed)]),
            &mut storage,
            &mut scratch,
        ),
        Err(OperatingError::ShapeMismatch),
    );
}

#[test]
fn cycle_and_compact_adjacency_preserve_parallel_edges_and_first_seen_vertices() {
    let source = table(2, &[&[9, 1], &[9, 1], &[1, 9], &[7, 7]]);
    let schemas = [InputSchema {
        kinds: &[CellKind::Number, CellKind::Number],
        tree: None,
    }];
    let mut storage = DerivedStorage::Unbuilt;
    let mut scratch = DerivedScratch::default();
    let cycles = CycleAdjacencyItem;
    let contract = cycles
        .prepare(
            &ViewRequest::CycleAdjacency {
                source: ColumnIndex(0),
                destination: ColumnIndex(1),
            },
            &schemas,
        )
        .unwrap();
    cycles
        .build(
            &contract,
            DerivedInputs::Rows(&[BorrowedRows::Computed(&source)]),
            &mut storage,
            &mut scratch,
        )
        .unwrap();
    let DerivedView::CycleAdjacency(graph) = cycles.view(&contract, &storage).unwrap() else {
        panic!("cycle view")
    };
    assert_eq!(graph.values, vec![number(9), number(1), number(7)]);
    assert_eq!(graph.starts, vec![0, 2, 3, 4]);
    assert_eq!(graph.leaving, vec![1, 1, 0, 2]);

    let adjacency = AdjacencyItem;
    let contract = adjacency
        .prepare(
            &ViewRequest::Adjacency {
                endpoints: [ColumnIndex(0), ColumnIndex(1)],
                keys: vec![],
            },
            &schemas,
        )
        .unwrap();
    let identity = GraphIdentity {
        population: Population {
            relation: Relation::SyntaxTree,
            kinds: &[CellKind::Number, CellKind::Number],
            rows: source.count.0,
            native_file: None,
            selected: None,
            positions: None,
        },
        endpoints: [ColumnIndex(0), ColumnIndex(1)],
        keys: vec![],
    };
    let mismatched_source = GraphSource {
        identity: identity.clone(),
        endpoints: [ColumnIndex(1), ColumnIndex(0)],
        orientation: Orientation::Forward,
    };
    assert_eq!(
        adjacency.build(
            &contract,
            DerivedInputs::Adjacency {
                rows: BorrowedRows::Computed(&source),
                source: mismatched_source
            },
            &mut storage,
            &mut scratch,
        ),
        Err(OperatingError::ShapeMismatch),
    );
    let source_identity = GraphSource {
        identity,
        endpoints: [ColumnIndex(0), ColumnIndex(1)],
        orientation: Orientation::Forward,
    };
    adjacency
        .build(
            &contract,
            DerivedInputs::Adjacency {
                rows: BorrowedRows::Computed(&source),
                source: source_identity,
            },
            &mut storage,
            &mut scratch,
        )
        .unwrap();
    let DerivedView::Adjacency(graph) = adjacency.view(&contract, &storage).unwrap() else {
        panic!("adjacency view")
    };
    assert_eq!(graph.vertices, vec![number(9), number(1), number(7)]);
    assert_eq!(graph.neighbors[0].len(), 4);
    let nine = graph.vertex_ids[&(0, number(9))] as usize;
    let neighbors =
        &graph.neighbors[0][graph.offsets[0][nine] as usize..graph.offsets[0][nine + 1] as usize];
    assert_eq!(neighbors, &[1, 1]);
}

#[test]
fn ordered_keys_sort_each_side_and_refuse_duplicate_order_within_a_group() {
    let left = table(2, &[&[5, 20], &[5, 10], &[7, 4]]);
    let right = table(2, &[&[5, 30], &[5, 25], &[7, 5]]);
    let schemas = [
        InputSchema {
            kinds: &[CellKind::Number, CellKind::Number],
            tree: None,
        },
        InputSchema {
            kinds: &[CellKind::Number, CellKind::Number],
            tree: None,
        },
    ];
    let request = ViewRequest::OrderedKeys {
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(0)],
        left_order: ColumnIndex(1),
        right_order: ColumnIndex(1),
    };
    let symbols = Symbols::default();
    let texts = crate::domain::table_texts::types::Texts {
        symbols: &symbols,
        source: crate::domain::table_texts::types::SourceView(&[]),
        paths: &[],
        arenas: &[],
    };
    let item = OrderedKeysItem;
    let contract = item.prepare(&request, &schemas).unwrap();
    let mut storage = DerivedStorage::Unbuilt;
    let mut scratch = DerivedScratch::default();
    item.build(
        &contract,
        DerivedInputs::Text {
            rows: &[
                BorrowedRows::Computed(&left),
                BorrowedRows::Computed(&right),
            ],
            texts: &texts,
        },
        &mut storage,
        &mut scratch,
    )
    .unwrap();
    let DerivedView::OrderedKeys(ordered) = item.view(&contract, &storage).unwrap() else {
        panic!("ordered view")
    };
    assert_eq!(
        ordered.left.iter().map(|row| row.order).collect::<Vec<_>>(),
        vec![Integer(10), Integer(20), Integer(4)]
    );
    assert_eq!(
        ordered
            .right
            .iter()
            .map(|row| row.order)
            .collect::<Vec<_>>(),
        vec![Integer(25), Integer(30), Integer(5)]
    );

    let duplicate = table(2, &[&[5, 10], &[5, 10]]);
    assert_eq!(
        item.build(
            &contract,
            DerivedInputs::Text {
                rows: &[
                    BorrowedRows::Computed(&duplicate),
                    BorrowedRows::Computed(&right)
                ],
                texts: &texts
            },
            &mut storage,
            &mut scratch,
        ),
        Err(OperatingError::DuplicateOrder),
    );
}
