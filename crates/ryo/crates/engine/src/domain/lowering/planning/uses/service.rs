//! The services of the `uses` part: what a use of a definition costs, from the definition's own plan for the columns the
//! use binds (`swarm/cut`, "a use costed from the definition's own plan per binding pattern"). Bound on a column, a use
//! runs the program the definition has with that column given: its steps and its rows are that program's, so a use by a
//! column that is no key of the definition is as dear as the definition's own plan says. Read with nothing bound, a
//! definition's rows are its table's, and making the table costs its whole plan once a file. Each is planned once however
//! many accesses ask it.

use crate::domain::lowering::costing::constants::{
    ANCESTORS, CLOSURE_STEP, CONSTANT_EQUAL_KEEPS, DESCENDANTS, MADE_ROWS, ONE, THINNED_DESCENDANTS,
};
use crate::domain::lowering::costing::service::{
    keyed, made_rows, planned_rows, plus, times, whole_rows,
};
use crate::domain::lowering::costing::types::{AccessCost, CellState, PlanEstimate, UseAsked};
use crate::domain::lowering::definitions::service::draft;
use crate::domain::lowering::definitions::types::{Drafts, RowEstimate};
use crate::domain::lowering::flattening::service::made_column;
use crate::domain::lowering::flattening::types::{
    AtomRows, EdgeColumns, EdgeRows, FlatAtom, FlatBranch, FlatTerm, FlatVariable, NearestOver,
    RowColumn,
};
use crate::domain::lowering::placement::patterns::service::given;
use crate::domain::lowering::planning::completing::service::completed;
use crate::domain::lowering::planning::service::planned_with;
use crate::domain::lowering::planning::types::{PlanningWith, UnitRole, UnitStart};
use crate::domain::steps::types::{ColumnMask, DefinitionId, MadeColumn, MadeTableId};
use std::collections::HashSet;

/// What a pattern's bound columns give a branch at its start: the variables of those columns, and the rows of one start,
/// a bound column that is a constant of the branch keeping the share an equality with a constant does.
fn started(branch: &FlatBranch, bound: &ColumnMask) -> (HashSet<FlatVariable>, PlanEstimate) {
    let constants = bound.0.iter().filter(|column| {
        matches!(
            branch.row.get(usize::from(column.0)),
            Some(FlatTerm::Text(_) | FlatTerm::Number(_))
        )
    });
    let rows = constants.fold(ONE, |rows, _| times(rows, CONSTANT_EQUAL_KEEPS));
    (given(branch, bound), rows)
}

/// What a definition asked by `bound` costs for one asking, by its own plan: the steps and the rows of its branches
/// together, each planned as a pattern's with the bound columns given, or as a pipeline's when nothing is bound. None
/// when a branch has no such program: the definition is then read as its table. Asked once: kept in `with.costs`.
fn planned_use(
    with: PlanningWith<'_>,
    definition: DefinitionId,
    bound: &ColumnMask,
) -> Option<AccessCost> {
    let draft = draft(with.drafts, definition)?;
    let asked = UseAsked {
        definition,
        node: draft.node,
        bound: bound.clone(),
    };
    if let Some(known) = with.costs.0.borrow().get(&asked) {
        return *known;
    }
    let mut cost = Some(AccessCost::default());
    for branch in &draft.branches {
        let (given, rows) = started(branch, bound);
        let start = UnitStart {
            given: &given,
            rows,
        };
        let plan = if bound.0.is_empty() {
            planned_with(with, branch, start, UnitRole::Pipeline)
        } else {
            completed(with, branch, start, UnitRole::Pattern, None)
        };
        cost = cost.zip(plan).map(|(cost, plan)| AccessCost {
            rows: plus(cost.rows, plan.rows),
            steps: plus(cost.steps, plus(plan.cost, ONE)),
            once: PlanEstimate(0),
        });
        if cost.is_none() {
            break;
        }
    }
    // A use by a key gives no more rows than the definition holds.
    let held = planned_rows(draft.rows);
    let cost = cost.map(|cost| AccessCost {
        rows: if bound.0.is_empty() {
            cost.rows
        } else {
            cost.rows.min(held)
        },
        ..cost
    });
    let _old = with.costs.0.borrow_mut().insert(asked, cost);
    cost
}

/// What making a definition's table costs, once a file: its whole plan's steps. A definition with no whole plan is taken to
/// cost its rows.
fn table_made(with: PlanningWith<'_>, definition: DefinitionId) -> PlanEstimate {
    let rows = draft(with.drafts, definition).map_or(RowEstimate(1), |draft| draft.rows);
    planned_use(with, definition, &ColumnMask::default())
        .map_or(planned_rows(rows), |cost| cost.steps)
}

/// What a use of a definition bound on `bound` costs for each row of its unit: the program of the definition's own plan
/// for those columns; or, when it has none, a read of its table by them, which gives `table_rows` and is made once.
pub(crate) fn used(
    with: PlanningWith<'_>,
    definition: DefinitionId,
    bound: &ColumnMask,
    table_rows: PlanEstimate,
) -> AccessCost {
    planned_use(with, definition, bound).unwrap_or_else(|| AccessCost {
        rows: table_rows,
        steps: plus(ONE, table_rows),
        once: table_made(with, definition),
    })
}

/// What a read of a definition's rows with no register costs: `rows` of its table for each row of the unit, the table
/// made once.
pub(crate) fn read(
    with: PlanningWith<'_>,
    definition: DefinitionId,
    rows: PlanEstimate,
) -> AccessCost {
    AccessCost {
        rows,
        steps: rows.max(ONE),
        once: table_made(with, definition),
    }
}

/// What a read of a made table costs: `rows` of it for each row of the unit, the table made once from an input of the
/// rows it is sized at.
pub(crate) fn made(drafts: &Drafts, table: MadeTableId, rows: PlanEstimate) -> AccessCost {
    AccessCost {
        rows,
        steps: rows.max(ONE),
        once: planned_rows(made_rows(drafts, table)),
    }
}

/// The rows one step over a closure's edges gives from an element: the edges' own plan with the column of the end walked
/// from, and the keys, bound; or, for edges read in place, the read by that column.
fn edge_rows(with: PlanningWith<'_>, edges: &EdgeColumns, from_source: bool) -> AccessCost {
    let from = if from_source {
        edges.source
    } else {
        edges.destination
    };
    let mut bound: Vec<MadeColumn> = edges.keys.iter().copied().chain([from]).collect();
    bound.sort_unstable();
    bound.dedup();
    match &edges.rows {
        EdgeRows::Definition(definition) => {
            let table = draft(with.drafts, *definition).map_or(RowEstimate(1), |draft| draft.rows);
            used(
                with,
                *definition,
                &ColumnMask(bound.into()),
                planned_rows(table),
            )
        }
        EdgeRows::Read {
            rows,
            cells,
            per_file,
        } => {
            let states: Vec<CellState> = (0..cells.len())
                .map(
                    |place| match (made_column(RowColumn(place)), cells.get(place)) {
                        (Some(column), _) if bound.contains(&column) => CellState::Register,
                        (_, Some(FlatTerm::Text(_) | FlatTerm::Number(_))) => CellState::Constant,
                        (_, Some(FlatTerm::File)) => CellState::File,
                        (_, Some(FlatTerm::Variable(_)) | None) => CellState::Open,
                    },
                )
                .collect();
            let rows = keyed(with.drafts, *rows, *per_file, &states).rows;
            AccessCost {
                rows,
                steps: rows.max(ONE),
                once: PlanEstimate(0),
            }
        }
    }
}

/// What a closure's walk costs from the end it is entered at: what it reaches by the rows one step over its edges gives
/// (about one: a chain toward a root; under a parent's children: edges that tests thin; else what lies under an
/// element), and for each element reached one step over the edges.
pub(crate) fn closure(
    with: PlanningWith<'_>,
    edges: &EdgeColumns,
    from_source: bool,
) -> AccessCost {
    let step = edge_rows(with, edges, from_source);
    let reach = if step.rows <= CLOSURE_STEP.0 {
        ANCESTORS
    } else if step.rows < CLOSURE_STEP.1 {
        THINNED_DESCENDANTS
    } else {
        DESCENDANTS
    };
    AccessCost {
        rows: reach,
        steps: times(reach, plus(step.steps, ONE)),
        once: step.once,
    }
}

/// How deep a view of a view is looked through for a walk.
const VIEWS_LOOKED_THROUGH: usize = 4;

/// Whether a definition's rows are a walk's own: a branch of it holds a closure or a climb over derived edges, or is a
/// view of such a definition (the one source of its rows is a use of it). Read whole, such a definition is every pair
/// its walk reaches in the file, which no estimate of the plan sees (`swarm/cut`: "a probe is preferred only when the
/// definition's sub-program walks no closure and no nearest-over-edges"). A definition that only asks such a walk by a
/// key, among other sources of rows, is not one: its rows are bounded by those sources.
pub(crate) fn walks_edges(drafts: &Drafts, definition: DefinitionId, depth: usize) -> bool {
    let Some(draft) = draft(drafts, definition) else {
        return false;
    };
    draft.branches.iter().any(|branch| {
        let walks = branch.atoms.iter().any(|atom| {
            matches!(
                atom,
                FlatAtom::Closure { .. }
                    | FlatAtom::Nearest {
                        over: NearestOver::Derived(_),
                        ..
                    }
            )
        });
        let mut sources = branch.atoms.iter().filter_map(|atom| match atom {
            FlatAtom::Rows { rows, .. } => Some(*rows),
            FlatAtom::Latest { .. }
            | FlatAtom::Decided { .. }
            | FlatAtom::Walk { .. }
            | FlatAtom::Nearest { .. }
            | FlatAtom::Closure { .. }
            | FlatAtom::Called { .. }
            | FlatAtom::Test { .. } => None,
        });
        let view = match (sources.next(), sources.next()) {
            (Some(AtomRows::Definition(used)), None) => {
                depth < VIEWS_LOOKED_THROUGH && walks_edges(drafts, used, depth + 1)
            }
            (
                Some(AtomRows::Definition(_) | AtomRows::Made(_) | AtomRows::Relation(_)) | None,
                _,
            ) => false,
        };
        walks || view
    })
}

/// Runs `planning` with the uses costed so far for the pack of `drafts`: what it costs is kept for the pack's next branch.
pub(crate) fn with_pack<T>(drafts: &Drafts, planning: impl FnOnce(PlanningWith<'_>) -> T) -> T {
    planning(PlanningWith {
        drafts,
        costs: &drafts.costs,
    })
}

/// The rows a step's branches are taken to give in a file together: each branch's own plan with nothing bound, as a
/// pipeline of the step's table. A branch with no plan counts one row.
#[must_use]
pub(crate) fn branches_estimated(drafts: &Drafts, branches: &[FlatBranch]) -> RowEstimate {
    let nothing = HashSet::new();
    let start = UnitStart {
        given: &nothing,
        rows: ONE,
    };
    let rows = with_pack(drafts, |with| {
        branches.iter().fold(PlanEstimate(0), |sum, branch| {
            let plan = planned_with(with, branch, start, UnitRole::Pipeline);
            plus(sum, plan.map_or(ONE, |plan| plan.rows))
        })
    });
    // A step's table is sized as any made table is: between the least and the greatest rows of one.
    whole_rows(rows.clamp(MADE_ROWS.0, MADE_ROWS.1))
}
