//! Declared populations are refused or kept exactly, never silently shortened.
use super::super::super::scratch_owner::test::{execute, hosted};
use super::super::super::*;
use super::super::fixtures::source;
use crate::domain::table_model::types::{SelectedRows, StoredSelection};
use ryo_relations::domain::relations::types::{ColumnsView, FactColumn};
#[test]
fn malformed_declared_population_is_refused_not_silently_shortened() {
    let table = Table {
        kinds: kinds(Relation::SyntaxTree).to_vec(),
        cells: vec![Cell::Node(ryo_relations::domain::relations::types::NodeId(
            0,
        ))],
        count: RowCount(2),
    };
    assert!(matches!(
        source(Rows::Computed(&table)),
        Err(OperatingError::ShapeMismatch)
    ));
}
#[test]
fn zero_width_bags_keep_their_declared_population() {
    let (result, _) = execute(
        Operation::Project {
            from: PlanInput::Step(StepId(0)),
            columns: vec![],
            distinct: RepeatedRows::Kept,
        },
        hosted(&[], vec![vec![], vec![], vec![]]),
        &mut Symbols::default(),
    );
    let output = result.unwrap();
    assert_eq!(output.count.0, 3);
    assert!(output.cells.is_empty());
}
#[test]
fn ragged_selected_fallback_cannot_admit_mismatched_projection_kinds() {
    let columns = [
        FactColumn(vec![0, 1]),
        FactColumn(vec![0]),
        FactColumn(vec![0, 0]),
        FactColumn(vec![0, 0]),
        FactColumn(vec![0, 0]),
    ];
    let rows = Rows::Selected(SelectedRows {
        source: ColumnsView {
            kinds: kinds(Relation::SyntaxTree),
            columns: &columns,
        },
        selection: Arc::new(StoredSelection {
            relation: Relation::SyntaxTree,
            rows: Arc::new(vec![0]),
            columns: vec![ColumnIndex(0)],
            kinds: vec![CellKind::Symbol],
            identity: Arc::new(Default::default()),
        }),
    });
    assert!(matches!(source(rows), Err(OperatingError::ShapeMismatch)));
}
