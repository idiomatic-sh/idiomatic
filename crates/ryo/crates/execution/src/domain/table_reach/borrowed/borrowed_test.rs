//! Differential traversal behavior over an actual prepared equality index.
use super::*;
use crate::domain::operator::contract::views::BorrowedRows;
use ryo_derived::domain::contract::types::{
    BuildOutcome, BuildWorkspace, BulkRows, DerivedFault, DerivedInputs, DerivedView, KeyIndex,
    RowCells, ViewRequest,
};
use ryo_relations::domain::relations::types::{CellKind, Integer};
use ryo_values::domain::rows::types::{RowCount, Table};

fn number(value: i64) -> Value {
    Value::Number(Integer(value))
}
struct Cells<'a>(&'a Table);
impl RowCells for Cells<'_> {
    fn bulk(&self) -> Option<BulkRows<'_>> {
        Some(BulkRows::Computed(self.0))
    }
    fn validate(&self) -> Result<(), DerivedFault> {
        if self.0.cells.len() == self.0.count.0 * self.0.kinds.len() {
            Ok(())
        } else {
            Err(DerivedFault::ShapeMismatch)
        }
    }
    fn kinds(&self) -> &[CellKind] {
        &self.0.kinds
    }
    fn count(&self) -> usize {
        self.0.count.0
    }
    fn cell(&self, row: usize, column: ColumnIndex) -> Option<Value> {
        self.0
            .cells
            .get(row * self.0.kinds.len() + column.0 as usize)
            .copied()
    }
}
fn with_edges(rows: &[[i64; 2]], run: impl FnOnce(&Table, &KeyIndex)) {
    let table = Table {
        kinds: vec![CellKind::Number; 2],
        count: RowCount(rows.len()),
        cells: rows
            .iter()
            .flat_map(|row| row.iter().copied().map(number))
            .collect(),
    };
    let port = (std::sync::Arc::new(
        ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
    ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>);
    let prepared = port.prepare(
        ViewRequest::Equality {
            columns: vec![ColumnIndex(0)],
        },
        vec![table.kinds.clone()],
    );
    let BuildOutcome::Built(payload) = port
        .build(
            &prepared,
            DerivedInputs::Rows(&Cells(&table)),
            &mut BuildWorkspace::default(),
            None,
        )
        .expect("valid equality build")
    else {
        panic!("small readable rows are supported")
    };
    let DerivedView::Equality(index) = port.view(&prepared, &payload).expect("equality view")
    else {
        panic!("prepared equality product")
    };
    run(&table, index);
}
fn view<'a>(table: &'a Table, index: &'a KeyIndex, columns: &'a [ColumnIndex]) -> IndexedRows<'a> {
    IndexedRows {
        #[cfg(feature = "diagnostic-row-progress")]
        progress: |_, _| {},
        rows: BorrowedRows::Computed(table),
        columns,
        index,
    }
}

#[test]
fn nearest_matches_first_admitted_breadth_vertex_for_branches_duplicates_cycles_and_ties() {
    for rows in [
        vec![[1, 2], [2, 3], [3, 4]],
        vec![[1, 2], [1, 3], [2, 4], [3, 5]],
        vec![[1, 3], [1, 2], [3, 4], [2, 4]],
        vec![[1, 2], [1, 2], [2, 3], [3, 1]],
        vec![[1, 1], [1, 2]],
        vec![],
    ] {
        with_edges(&rows, |table, index| {
            let columns = [ColumnIndex(0)];
            let edges = view(table, index, &columns);
            for start in [1, 2, 9] {
                let mut full = Walk::default();
                reach_breadth(&edges, ColumnIndex(1), number(start), &mut full);
                for admitted in [vec![], vec![1], vec![2], vec![3, 2], vec![4, 5]] {
                    let wanted = full
                        .reached
                        .iter()
                        .copied()
                        .find(|value| admitted.iter().any(|n| number(*n) == *value));
                    let mut nearest = Walk::default();
                    reach_nearest(
                        &edges,
                        ColumnIndex(1),
                        number(start),
                        &mut nearest,
                        |value| Ok::<_, ()>(admitted.iter().any(|n| number(*n) == value)),
                    )
                    .expect("infallible membership");
                    assert_eq!(
                        nearest.reached,
                        wanted.into_iter().collect::<Vec<_>>(),
                        "rows={rows:?},start={start},domain={admitted:?}"
                    );
                }
            }
        });
    }
}

#[test]
fn reused_walk_nearest_hit_then_no_path_then_full_walk_clears_previous_work() {
    with_edges(&[[1, 2], [1, 3], [2, 4], [3, 5]], |table, index| {
        let columns = [ColumnIndex(0)];
        let edges = view(table, index, &columns);
        let mut walk = Walk::default();
        reach_nearest(&edges, ColumnIndex(1), number(1), &mut walk, |value| {
            Ok::<_, ()>(value == number(3))
        })
        .expect("hit");
        assert_eq!(walk.reached, vec![number(3)]);
        assert_eq!(walk.visited.len(), 2, "stop before descendants are visited");
        assert!(
            !walk.next.is_empty(),
            "control leaves queued descendants to be cleared"
        );
        reach_nearest(&edges, ColumnIndex(1), number(99), &mut walk, |_| {
            Err("must not validate unreachable membership")
        })
        .expect("no path");
        assert!(walk.reached.is_empty());
        reach_breadth(&edges, ColumnIndex(1), number(1), &mut walk);
        assert_eq!(
            walk.reached,
            vec![number(2), number(3), number(4), number(5)]
        );
    });
}

#[test]
fn admission_fault_is_deferred_until_first_reached_vertex_and_stops_further_admission() {
    with_edges(&[[1, 2], [2, 3]], |table, index| {
        let columns = [ColumnIndex(0)];
        let edges = view(table, index, &columns);
        let mut walk = Walk::default();
        let mut called = Vec::new();
        let result = reach_nearest(&edges, ColumnIndex(1), number(1), &mut walk, |value| {
            called.push(value);
            Err("wrong membership view")
        });
        assert_eq!(result, Err("wrong membership view"));
        assert_eq!(called, vec![number(2)]);
        assert!(walk.reached.is_empty());
    });
}

#[test]
fn missing_destination_cells_are_skipped_as_in_full_traversal() {
    with_edges(&[[1, 2], [2, 3]], |table, index| {
        let mut ragged = table.clone();
        ragged.cells.pop();
        let columns = [ColumnIndex(0)];
        let edges = view(&ragged, index, &columns);
        let mut full = Walk::default();
        reach_breadth(&edges, ColumnIndex(1), number(1), &mut full);
        assert_eq!(full.reached, vec![number(2)]);
        let mut nearest = Walk::default();
        reach_nearest(&edges, ColumnIndex(1), number(1), &mut nearest, |value| {
            Ok::<_, ()>(value == number(3))
        })
        .expect("missing destination is not a new error");
        assert!(nearest.reached.is_empty());
    });
}
