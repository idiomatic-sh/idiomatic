//! The graph cases' rows and steps: edges of file 0 by node position, the closure, the join a closure is fused into, its
//! demand, and a select that faults on a row.
use super::super::super::fixtures::TextResources;
use crate::domain::physical_plan::fixtures::Step;
use crate::domain::physical_plan::types::PlanInput;
use ryo_relations::domain::relations::fixtures::{kinds, node_id};
use ryo_relations::domain::relations::types::{FileId, Integer, Position, Relation, Value as Cell};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input as StepInput, JoinColumn, Operand, PlanComparison,
    ReachLookup, Side, Test,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::{RowCount, Table};

/// The node at position `at` of file 0.
pub(super) fn node(at: u32) -> Cell {
    Cell::Node(node_id(FileId(0), Position(at)))
}

/// The edge rows `parent -> child`, each with its ordinal, the field `f` and file 0.
pub(super) fn edges(pairs: &[(u32, u32, i64)], texts: &TextResources) -> Table {
    let mut cells = Vec::with_capacity(pairs.len() * kinds(EDGES).len());
    for (parent, child, ordinal) in pairs {
        cells.extend([
            node(*parent),
            node(*child),
            Cell::Number(Integer(*ordinal)),
            texts.symbol("f"),
            Cell::File(FileId(0)),
        ]);
    }
    Table {
        kinds: kinds(EDGES).to_vec(),
        cells,
        count: RowCount(pairs.len()),
    }
}

/// A cell as the case states it: a node by its position, a number as itself.
pub(super) fn stated(cell: Cell) -> i64 {
    if let Cell::Node(id) = cell {
        i64::from(ryo_relations::domain::relations::service::node_position(id).0)
    } else if let Cell::Number(Integer(number)) = cell {
        number
    } else {
        panic!("a case states nodes and numbers only, not {cell:?}")
    }
}

/// The closure of the edges from every source.
pub(super) fn closure(edges: StepInput, input: PlanInput) -> Step {
    Step {
        operation: Operation::Closure {
            edges,
            keys: Vec::new(),
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
        },
        inputs: vec![input],
    }
}

/// A join of the edges, as left rows, with `right` on the left's parent and, with `residual`, its child, answering the
/// left's parent and the right's destination; `reach` when the closure was fused into it.
pub(super) fn join(
    right: (StepInput, PlanInput),
    residual: bool,
    reach: Option<ReachLookup>,
) -> Step {
    let (left_on, right_on) = if residual {
        (
            vec![ColumnIndex(0), ColumnIndex(1)],
            vec![ColumnIndex(0), ColumnIndex(1)],
        )
    } else {
        (vec![ColumnIndex(0)], vec![ColumnIndex(0)])
    };
    Step {
        operation: Operation::Join {
            left: StepInput::Relation(EDGES),
            right: right.0,
            left_on,
            right_on,
            output: vec![
                JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(0),
                },
                JoinColumn {
                    side: Side::Right,
                    column: ColumnIndex(1),
                },
            ],
            filter: Vec::new(),
            view: Vec::new(),
            fan: ryo_values::domain::plan_parts::types::KeyFan::Shared,
            reach,
        },
        inputs: vec![PlanInput::Relation(EDGES), right.1],
    }
}

/// The join `step` with its left rows read from `left` instead.
pub(super) fn left_of(mut step: Step, left: (StepInput, PlanInput)) -> Step {
    if let Operation::Join { left: read, .. } = &mut step.operation {
        *read = left.0;
    }
    if let Some(input) = step.inputs.first_mut() {
        *input = left.1;
    }
    step
}

/// The closure's demand as the query planner fuses it: no partition, its source then its destination.
pub(super) fn reach() -> ReachLookup {
    ReachLookup {
        source: ColumnIndex(0),
        destination: ColumnIndex(1),
        keys: Vec::new(),
        columns: vec![ColumnIndex(0), ColumnIndex(1)],
    }
}

/// The rows of `relation` whose field (column 3) self-matches as a regex: a field that is no pattern faults on its row.
pub(super) fn self_matching(relation: Relation) -> Step {
    Step {
        operation: Operation::Select {
            from: StepInput::Relation(relation),
            comparisons: vec![PlanComparison {
                test: Test::Matches,
                left: Operand::Column(ColumnIndex(3)),
                right: Operand::Column(ColumnIndex(3)),
                origin: ComparisonOrigin::Unrecorded,
            }],
            kept: Vec::new(),
        },
        inputs: vec![PlanInput::Relation(relation)],
    }
}

/// The relation the cases read edges from: parent, child, ordinal, field, file.
pub(super) const EDGES: Relation = Relation::SemanticChild;
