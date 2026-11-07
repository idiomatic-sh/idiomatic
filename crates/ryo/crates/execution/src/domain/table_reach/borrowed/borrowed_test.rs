//! Differential traversal behavior over an actual prepared equality index.
use crate::outbound::derived::interface::DerivedPort as _;
use super::*;
use ryo_relations::domain::relations::types::Value as Cell;

use crate::domain::table_model::types::RepresentationInput;
use ryo_derived::domain::contract::types::BuildOutcome;
use ryo_derived::domain::contract::types::BuildWorkspace;
use ryo_derived::domain::row_cells::types::BulkRows;
use ryo_derived::domain::fault::types::DerivedFault;
use ryo_derived::domain::contract::types::BuildInputs;
use ryo_derived::domain::contract::types::DerivedView;
use ryo_derived::domain::equality::types::KeyIndex;
use ryo_derived::domain::row_cells::types::RowCells;
use ryo_derived::domain::request::types::DerivedRequest;
use ryo_derived::inbound::use_cases::build::interface::BuildUseCase;
use ryo_derived::inbound::use_cases::prepare_derived::interface::PrepareDerivedUseCase;
use ryo_derived::inbound::use_cases::view::interface::ViewUseCase;
use ryo_relations::domain::relations::types::{CellKind, Integer};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::rows::types::{RowCount, Table};

fn membership(held: bool) -> SetMembership {
    if held {
        SetMembership::In
    } else {
        SetMembership::Out
    }
}
fn number(value: i64) -> Cell {
    Cell::Number(Integer(value))
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
    fn count(&self) -> ryo_values::domain::rows::types::RowCount {
        self.0.count
    }
    fn cell(&self, row: ryo_values::domain::rows::types::RowIndex, column: ColumnIndex) -> Option<Cell> {
        self.0
            .cells
            .get(row.0 * self.0.kinds.len() + column.0 as usize)
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
    let port = std::sync::Arc::new(
        crate::outbound::derived::factory::semantics(),
    );
    let prepared = port.prepare(
        DerivedRequest::Equality {
            columns: vec![ColumnIndex(0)],
        },
        vec![table.kinds.clone()],
    );
    let BuildOutcome::Built(payload) = port
        .build(
            &prepared,
            BuildInputs::Rows(&Cells(&table)),
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
        progress: &crate::domain::operator::contract::views::IgnoredProgress,
        rows: RepresentationInput::Computed(table),
        columns,
        index,
    }
}

fn nearest_matches_first_admitted_breadth_vertex_for_branches_duplicates_cycles_and_ties() -> Value {
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
                        |value| Ok::<_, ()>(membership(admitted.iter().any(|n| number(*n) == value))),
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
    1
}

fn reused_walk_nearest_hit_then_no_path_then_full_walk_clears_previous_work() -> Value {
    with_edges(&[[1, 2], [1, 3], [2, 4], [3, 5]], |table, index| {
        let columns = [ColumnIndex(0)];
        let edges = view(table, index, &columns);
        let mut walk = Walk::default();
        reach_nearest(&edges, ColumnIndex(1), number(1), &mut walk, |value| {
            Ok::<_, ()>(membership(value == number(3)))
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
    1
}

fn admission_fault_is_deferred_until_first_reached_vertex_and_stops_further_admission() -> Value {
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
    1
}

fn missing_destination_cells_are_skipped_as_in_full_traversal() -> Value {
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
            Ok::<_, ()>(membership(value == number(3)))
        })
        .expect("missing destination is not a new error");
        assert!(nearest.reached.is_empty());
    });
    1
}

/// A scenario: what a row asks of the part.
type Input = fn() -> Value;
/// What a scenario answers: a yes as one.
type Value = usize;
/// A scenario that checks its own claims never fails as a whole.
type Failure = std::convert::Infallible;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("nearest_matches_first_admitted_breadth_vertex_for_branches_duplicates_cycles_and_ties"),
        word: Word::Succeeds,
        input: || nearest_matches_first_admitted_breadth_vertex_for_branches_duplicates_cycles_and_ties,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("reused_walk_nearest_hit_then_no_path_then_full_walk_clears_previous_work"),
        word: Word::Succeeds,
        input: || reused_walk_nearest_hit_then_no_path_then_full_walk_clears_previous_work,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("admission_fault_is_deferred_until_first_reached_vertex_and_stops_further_admission"),
        word: Word::Succeeds,
        input: || admission_fault_is_deferred_until_first_reached_vertex_and_stops_further_admission,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("missing_destination_cells_are_skipped_as_in_full_traversal"),
        word: Word::Succeeds,
        input: || missing_destination_cells_are_skipped_as_in_full_traversal,
        expect: Expect::Value(1),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(scenario())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
