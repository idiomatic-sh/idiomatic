//! The support of the `borrowed` cases.
use super::super::super::service::*;
use super::super::super::types::*;
use crate::domain::table_model::types::RepresentationInput;
use crate::domain::table_reach::types::Walk;
use ryo_derived::domain::contract::types::BuildInputs;
use ryo_derived::domain::contract::types::BuildOutcome;
use ryo_derived::domain::contract::types::BuildWorkspace;
use ryo_derived::domain::contract::types::DerivedView;
use ryo_derived::domain::equality::types::KeyIndex;
use ryo_derived::domain::fault::types::DerivedFault;
use ryo_derived::domain::request::types::DerivedRequest;
use ryo_derived::domain::row_cells::types::BulkRows;
use ryo_derived::domain::row_cells::types::RowCells;
use ryo_operators::domain::table_scratch::types::SetMembership;
use ryo_relations::domain::relations::types::Value as Cell;
use ryo_relations::domain::relations::types::{CellKind, Integer};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, Table};
use std::borrow::Cow;

pub(super) fn membership(held: bool) -> SetMembership {
    if held {
        SetMembership::In
    } else {
        SetMembership::Out
    }
}

pub(super) fn number(value: i64) -> Cell {
    Cell::Number(Integer(value))
}

pub(super) struct Cells<'a>(pub(super) &'a Table);

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
    fn count(&self) -> RowCount {
        self.0.count
    }
    fn cell(
        &self,
        row: ryo_values::domain::rows::types::RowIndex,
        column: ColumnIndex,
    ) -> Option<Cell> {
        self.0
            .cells
            .get(
                row.0 * self.0.kinds.len()
                    + usize::try_from(column.0).expect("a column index fits in usize"),
            )
            .copied()
    }
}

pub(super) fn with_edges<T>(rows: &[[i64; 2]], run: impl FnOnce(&Table, &KeyIndex) -> T) -> T {
    let table = Table {
        kinds: vec![CellKind::Number; 2],
        count: RowCount(rows.len()),
        cells: rows
            .iter()
            .flat_map(|row| row.iter().copied().map(number))
            .collect(),
    };
    let port = crate::domain::derived_views::fixtures::semantics();
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
    run(&table, index)
}

pub(super) fn view<'a>(
    table: &'a Table,
    index: &'a KeyIndex,
    columns: &'a [ColumnIndex],
) -> IndexedRows<'a> {
    IndexedRows {
        #[cfg(feature = "diagnostic-row-progress")]
        progress: &crate::domain::operating::types::IgnoredProgress,
        rows: RepresentationInput::Computed(table),
        columns,
        index,
    }
}

/// What a row walks: the edges and the walk asked of them.
#[derive(Clone, Copy, Debug)]
pub(super) enum Traversal {
    /// Nearest walks from each start under each admitted set, over each edge table, against the breadth walk's
    /// first admitted vertex.
    NearestAgainstBreadth(&'static [&'static [[i64; 2]]]),
    /// One walk value reused: a nearest hit at 3 from 1, a start with no path, then a full breadth walk from 1.
    ReusedWalk(&'static [[i64; 2]]),
    /// A nearest walk from 1 whose admission check fails.
    FailingAdmission(&'static [[i64; 2]]),
    /// Edges whose last destination cell is missing: a breadth walk and a nearest walk for 3, both from 1.
    MissingDestination(&'static [[i64; 2]]),
}

/// What a walk observed.
#[derive(Debug, PartialEq)]
pub(super) enum Observed {
    /// How many (edges, start, admitted set) combinations ran, and in how many the nearest walk reached exactly
    /// the breadth walk's first admitted vertex.
    Agreement {
        combinations: usize,
        agreeing: usize,
    },
    /// The reused walk at each step.
    Reused {
        /// What the nearest hit reached.
        hit: Cow<'static, [Cell]>,
        /// The hit's error.
        hit_error: Option<()>,
        /// How many vertices the hit visited.
        visited_at_hit: usize,
        /// Whether the hit left queued descendants.
        queued_at_hit: bool,
        /// What the walk from a start with no path reached.
        no_path: Cow<'static, [Cell]>,
        /// The error of the walk with no path.
        no_path_error: Option<&'static str>,
        /// What the full breadth walk reached after both.
        full: Cow<'static, [Cell]>,
    },
    /// The failing admission.
    Fault {
        /// The error the walk returned.
        error: Option<&'static str>,
        /// The vertices admission was asked about.
        asked: Cow<'static, [Cell]>,
        /// What the walk reached.
        reached: Cow<'static, [Cell]>,
    },
    /// The ragged table's walks.
    Ragged {
        /// What the breadth walk reached.
        full: Cow<'static, [Cell]>,
        /// What the nearest walk reached.
        nearest: Cow<'static, [Cell]>,
        /// The nearest walk's error.
        error: Option<()>,
    },
}

fn admitted_in(admitted: &[i64], value: Cell) -> bool {
    admitted.iter().any(|n| number(*n) == value)
}

/// Runs a row's walk.
pub(super) fn traverse(traversal: Traversal) -> Observed {
    match traversal {
        Traversal::NearestAgainstBreadth(tables) => nearest_against_breadth(tables),
        Traversal::ReusedWalk(edges) => reused_walk(edges),
        Traversal::FailingAdmission(edges) => failing_admission(edges),
        Traversal::MissingDestination(edges) => missing_destination(edges),
    }
}

fn nearest_against_breadth(tables: &[&[[i64; 2]]]) -> Observed {
    let mut combinations = 0;
    let mut agreeing = 0;
    for rows in tables {
        with_edges(rows, |table, index| {
            let columns = [ColumnIndex(0)];
            let edges = view(table, index, &columns);
            for start in [1, 2, 9] {
                let mut full = Walk::default();
                reach_breadth(&edges, ColumnIndex(1), number(start), &mut full);
                for admitted in [&[][..], &[1], &[2], &[3, 2], &[4, 5]] {
                    let wanted = full
                        .reached
                        .iter()
                        .copied()
                        .find(|value| admitted_in(admitted, *value));
                    let mut nearest = Walk::default();
                    let answered = reach_nearest(
                        &edges,
                        ColumnIndex(1),
                        number(start),
                        &mut nearest,
                        |value| Ok::<_, ()>(membership(admitted_in(admitted, value))),
                    );
                    combinations += 1;
                    if answered == Ok(())
                        && nearest.reached == wanted.into_iter().collect::<Vec<_>>()
                    {
                        agreeing += 1;
                    }
                }
            }
        });
    }
    Observed::Agreement {
        combinations,
        agreeing,
    }
}

fn reused_walk(rows: &[[i64; 2]]) -> Observed {
    with_edges(rows, |table, index| {
        let columns = [ColumnIndex(0)];
        let edges = view(table, index, &columns);
        let mut walk = Walk::default();
        let hit = reach_nearest(&edges, ColumnIndex(1), number(1), &mut walk, |value| {
            Ok::<_, ()>(membership(value == number(3)))
        });
        let hit_error = hit.err();
        let hit = walk.reached.clone();
        let visited_at_hit = walk.visited.len();
        let queued_at_hit = !walk.next.is_empty();
        let no_path = reach_nearest(&edges, ColumnIndex(1), number(99), &mut walk, |_| {
            Err("must not validate unreachable membership")
        });
        let no_path_error = no_path.err();
        let no_path = walk.reached.clone();
        reach_breadth(&edges, ColumnIndex(1), number(1), &mut walk);
        Observed::Reused {
            hit: Cow::Owned(hit),
            hit_error,
            visited_at_hit,
            queued_at_hit,
            no_path: Cow::Owned(no_path),
            no_path_error,
            full: Cow::Owned(walk.reached),
        }
    })
}

fn failing_admission(rows: &[[i64; 2]]) -> Observed {
    with_edges(rows, |table, index| {
        let columns = [ColumnIndex(0)];
        let edges = view(table, index, &columns);
        let mut walk = Walk::default();
        let mut asked = Vec::new();
        let result = reach_nearest(&edges, ColumnIndex(1), number(1), &mut walk, |value| {
            asked.push(value);
            Err("wrong membership view")
        });
        Observed::Fault {
            error: result.err(),
            asked: Cow::Owned(asked),
            reached: Cow::Owned(walk.reached),
        }
    })
}

fn missing_destination(rows: &[[i64; 2]]) -> Observed {
    with_edges(rows, |table, index| {
        let mut ragged = table.clone();
        ragged.cells.truncate(ragged.cells.len().saturating_sub(1));
        let columns = [ColumnIndex(0)];
        let edges = view(&ragged, index, &columns);
        let mut full = Walk::default();
        reach_breadth(&edges, ColumnIndex(1), number(1), &mut full);
        let mut nearest = Walk::default();
        let error = reach_nearest(&edges, ColumnIndex(1), number(1), &mut nearest, |value| {
            Ok::<_, ()>(membership(value == number(3)))
        })
        .err();
        Observed::Ragged {
            full: Cow::Owned(full.reached),
            nearest: Cow::Owned(nearest.reached),
            error,
        }
    })
}
