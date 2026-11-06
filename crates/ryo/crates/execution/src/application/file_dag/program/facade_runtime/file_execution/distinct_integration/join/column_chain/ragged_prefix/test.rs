//! A ragged child normalizes its prefix before the column chain, and an empty primary builds no index.
use super::super::super::capability::fixtures::Witness;
use super::super::super::*;
use super::super::fixtures::{plan, sources, stored};
use ryo_language::domain::symbols::service::intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::terms::types::NameText;
use std::sync::atomic::Ordering;
#[test]
fn ragged_semantic_child_normalizes_its_complete_prefix_before_column_chain_fallback() {
    let mut symbols = Symbols::default();
    let body = intern_derived(&mut symbols, ByteSlice(b""), NameText("body"));
    let other = intern_derived(&mut symbols, ByteSlice(b""), NameText("other"));
    let parent = Cell::Node(node_id(FileId(0), Position(1)));
    let child = Cell::Node(node_id(FileId(0), Position(2)));
    let function = stored(
        Relation::Function,
        Table {
            kinds: kinds(Relation::Function).to_vec(),
            count: RowCount(1),
            cells: vec![
                parent,
                Cell::Symbol(other),
                Cell::File(FileId(0)),
                Cell::Span(Span {
                    start: Offset(0),
                    end: Offset(1),
                }),
            ],
        },
    );
    let mut right = stored(
        Relation::SemanticChild,
        Table {
            kinds: kinds(Relation::SemanticChild).to_vec(),
            count: RowCount(2),
            cells: [0, 1]
                .into_iter()
                .flat_map(|ordinal| {
                    [
                        parent,
                        child,
                        Cell::Number(Integer(ordinal)),
                        Cell::Symbol(body),
                        Cell::File(FileId(0)),
                    ]
                })
                .collect(),
        },
    );
    // Keep the provider physically ragged. No fixture decoder, truncation of
    // other columns or rebuilt logical table may perform owner normalization.
    right.columns[2].0.truncate(1);
    assert_eq!(
        right
            .columns
            .iter()
            .map(|column| column.0.len())
            .collect::<Vec<_>>(),
        [2, 2, 1, 2, 2]
    );
    let witness = Arc::new(Witness::default());
    let actual = execute(
        plan(),
        [function, right],
        &mut symbols,
        b"",
        false,
        witness.clone(),
        Faults::default(),
    )
    .unwrap();
    assert!(
        actual.column_prepared,
        "the original authored chain remains a real compiled candidate"
    );
    assert_eq!(
        witness.columns.load(Ordering::Relaxed),
        0,
        "owner-normalized computed RHS must decline the stored column-cell alternative"
    );
    assert!(
        witness.calls.load(Ordering::Relaxed) > 0,
        "the actual registered Join handles the normalized population"
    );
    let table = actual.result.unwrap();
    assert_eq!(table.count, RowCount(1));
    assert_eq!(table.cells, [parent, Cell::File(FileId(0)), child]);
    assert!(actual.frontier_errors.is_empty());
    assert_eq!(actual.frontiers.len(), 1);
    assert_eq!(
        actual.frontiers[0],
        Table {
            kinds: vec![CellKind::Node, CellKind::File],
            count: RowCount(1),
            cells: vec![parent, Cell::File(FileId(0))],
        }
    );
}
#[test]
fn empty_column_primary_admits_rhs_and_metadata_without_building_an_index() {
    for bad_rhs in [
        None,
        Some(OperatingError::NotRead),
        Some(OperatingError::ShapeMismatch),
    ] {
        let mut symbols = Symbols::default();
        let [mut left, mut right] = sources(&mut symbols, 1, true);
        for column in &mut left.columns {
            column.0.clear();
        }
        let faults = if bad_rhs == Some(OperatingError::ShapeMismatch) {
            right.columns.pop(); // Actual malformed metadata, not a fabricated refusal.
            Faults::default()
        } else {
            Faults {
                right: bad_rhs.clone(),
                ..Faults::default()
            }
        };
        let witness = Arc::new(Witness::default());
        let actual = execute(
            plan(),
            [left, right],
            &mut symbols,
            b"",
            false,
            witness.clone(),
            faults,
        )
        .unwrap();
        assert!(
            actual.column_prepared,
            "the real compiled column strategy is installed"
        );
        if let Some(error) = bad_rhs {
            assert_eq!(actual.result, Err(error));
        } else {
            let table = actual.result.unwrap();
            assert_eq!(table.count, RowCount(0));
            assert_eq!(
                table.kinds,
                [CellKind::Node, CellKind::File, CellKind::Node]
            );
        }
        assert_eq!(witness.columns.load(Ordering::Relaxed), 0);
        assert!(
            actual.derived_builds.is_empty(),
            "no equality, graph or other payload is built for an empty primary"
        );
        assert!(actual.attempts.iter().any(|attempt| matches!(
            attempt,
            SourceAttempt::Available(1) | SourceAttempt::Refused(1, _)
        )));
    }
}
