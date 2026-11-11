//! File partitions keep identical node values separate and deliver rows in exact order.
use super::super::fixtures::{column, execute_request, host, hosted_plan, numbers, operation};
use ryo_execution_domain::domain::operating::fixtures::ExecutionObservation;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, FileId, Integer, Offset, Relation, Span, Value as Cell},
};
use ryo_values::domain::{
    plan_parts::types::Input as PlanInput,
    plans::types::Operation,
    rows::types::{RowCount, Table},
};
#[test]
fn file_partition_keys_keep_identical_node_values_separate() {
    use ryo_relations::domain::relations::types::NodeId;
    let n = |at| Cell::Node(NodeId(at));
    let edge_rows = [[1, 2, 10], [2, 3, 20], [1, 4, 20]].map(|[a, b, file]| {
        vec![
            n(a),
            n(b),
            Cell::Number(Integer(0)),
            Cell::Symbol(ryo_relations::domain::terms::types::Symbol(0)),
            Cell::File(FileId(file as u32)),
        ]
    });
    let root_rows = [10, 20].map(|file| {
        vec![
            n(1),
            Cell::File(FileId(file)),
            Cell::Span(Span {
                start: Offset(0),
                end: Offset(0),
            }),
            Cell::Number(Integer(1)),
            Cell::Number(Integer(1)),
            Cell::Number(Integer(0)),
            Cell::Number(Integer(0)),
            Cell::Number(Integer(0)),
            Cell::Number(Integer(0)),
        ]
    });
    let providers = [
        (
            Relation::SemanticChild,
            Table {
                kinds: kinds(Relation::SemanticChild).to_vec(),
                cells: edge_rows.into_iter().flatten().collect(),
                count: RowCount(3),
            },
        ),
        (
            Relation::SourceLocation,
            Table {
                kinds: kinds(Relation::SourceLocation).to_vec(),
                cells: root_rows.into_iter().flatten().collect(),
                count: RowCount(2),
            },
        ),
    ];
    let mut request = hosted_plan(operation(&[1], &[4]), 0, 1);
    request.steps[0].operation = Operation::Select {
        from: PlanInput::Relation(Relation::SourceLocation),
        comparisons: vec![],
        kept: vec![column(0), column(1)],
    };
    // The edge identity is a real projected node column, rather than a synthetic provider schema.
    request.steps[1].operation = Operation::Select {
        from: PlanInput::Relation(Relation::SemanticChild),
        comparisons: vec![],
        kept: vec![column(0), column(1), column(2), column(0), column(4)],
    };
    let out = execute_request(
        request,
        &[],
        &providers,
        &mut Symbols::default(),
        b"",
        false,
        &mut ExecutionObservation::default(),
    )
    .unwrap();
    assert_eq!(out.count, RowCount(2));
    assert_eq!(
        out.kinds,
        [
            CellKind::File,
            CellKind::Node,
            CellKind::Node,
            CellKind::Node,
            CellKind::Node,
            CellKind::Number
        ]
    );
    assert_eq!(
        out.cells,
        vec![
            Cell::File(FileId(10)),
            n(1),
            n(1),
            n(2),
            n(1),
            Cell::Number(Integer(0)),
            Cell::File(FileId(20)),
            n(1),
            n(1),
            n(4),
            n(1),
            Cell::Number(Integer(0))
        ]
    );
    assert!(!out.cells.chunks_exact(6).any(|row| row[3] == n(3)));
}
#[test]
fn row_delivery_and_collected_visits_keep_exact_order_and_duplicates() {
    let hosts = [
        host(&numbers(1, &[&[1], &[1]]), "roots"),
        host(
            &numbers(4, &[&[1, 2, 1, 12], &[1, 3, 0, 13], &[3, 1, 0, 31]]),
            "edges",
        ),
    ];
    let request = hosted_plan(operation(&[], &[]), 0, 1);
    let collected = execute_request(
        request.clone(),
        &hosts,
        &[],
        &mut Symbols::default(),
        b"",
        false,
        &mut ExecutionObservation::default(),
    )
    .unwrap();
    let emitted = execute_request(
        request,
        &hosts,
        &[],
        &mut Symbols::default(),
        b"",
        true,
        &mut ExecutionObservation::default(),
    )
    .unwrap();
    assert_eq!(collected.count, RowCount(3));
    assert_eq!(emitted.count, collected.count);
    assert_eq!(emitted.cells, collected.cells);
    assert_eq!(
        collected.cells,
        [1, 1, 3, 13, 0, 1, 3, 1, 31, 1, 1, 1, 2, 12, 2].map(|n| Cell::Number(Integer(n)))
    );
}
