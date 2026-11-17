//! The worklist's authored rows and expectation: root 1 holds state 0 and raises 2 to state 1, and 2 records its pop through
//! an edge to itself joining state 0.
use super::expectations::{Expected, cells};
use super::values::{DOMAIN, FROM, RIGHT, matrix, n};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::Table;

/// The worklist, its inputs (roots, edges, lattice) and the rows it answers.
#[must_use]
pub fn fixture() -> (Operation, Vec<Table>, Vec<Vec<Expected>>) {
    let rows: Vec<Vec<Value>> = vec![
        vec![n(1), n(1), n(2), n(0), n(0), n(1)],
        vec![n(1), n(2), n(2), n(0), n(1), n(1)],
    ];
    (
        Operation::Worklist {
            from: FROM,
            edges: RIGHT,
            lattice: DOMAIN,
            root: ColumnIndex(0),
            state: ColumnIndex(1),
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
            order: ColumnIndex(2),
            before: ColumnIndex(3),
            after: ColumnIndex(4),
            lower: ColumnIndex(0),
            upper: ColumnIndex(1),
            root_keys: vec![],
            edge_keys: vec![],
        },
        vec![
            matrix(&[&[1, 0]]),
            matrix(&[&[1, 2, 0, 0, 1], &[2, 2, 0, 1, 0]]),
            matrix(&[&[0, 1]]),
        ],
        cells(rows),
    )
}
