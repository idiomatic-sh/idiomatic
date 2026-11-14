//! Test support of the `physical_plan` concept: the steps of a case's program, and the case vocabulary the catalogs and the
//! rewrites are checked over.

use crate::domain::physical_plan::types::{PlanInput, PlanNodeId};
use ryo_relations::domain::relations::types::{Integer, Relation};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Direction, Input as StepInput, JoinColumn, LiteralText, Operand,
    PlanComparison, QueryLiteral, ReachLookup, RepeatedRows, Side, StepId, Test,
};
use ryo_values::domain::plans::types::Operation;

/// The relation the case vocabulary reads: refinement (column 2) and name (column 3) are texts.
pub const DEFINES: Relation = Relation::Defines;

/// One step of a case's program: its operation and what each of its inputs reads.
#[derive(Clone, Debug)]
pub struct Step {
    /// The operation.
    pub operation: Operation,
    /// What its inputs read, in the order its kind reads them.
    pub inputs: Vec<PlanInput>,
}

/// A comparison of column `column` with the written text `text`.
fn equal(column: u32, text: &str) -> PlanComparison {
    PlanComparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(column)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
        origin: ComparisonOrigin::Unrecorded,
    }
}

/// The step of `operation` over `inputs`.
fn step(operation: Operation, inputs: &[PlanInput]) -> Step {
    Step {
        operation,
        inputs: inputs.to_vec(),
    }
}

/// The input of an operation that reads node `at`.
fn node(at: u32) -> (StepInput, PlanInput) {
    (StepInput::Step(StepId(at)), PlanInput::Node(PlanNodeId(at)))
}

/// The input of an operation that reads `DEFINES`.
fn defines() -> (StepInput, PlanInput) {
    (StepInput::Relation(DEFINES), PlanInput::Relation(DEFINES))
}

/// A join of `left` with `right` on one key each, answering a left and a right column, its right rows kept by `filter`.
fn join(
    left: (StepInput, PlanInput),
    right: (StepInput, PlanInput),
    on: [u32; 2],
    filter: Vec<PlanComparison>,
) -> Step {
    step(
        Operation::Join {
            left: left.0,
            right: right.0,
            left_on: vec![ColumnIndex(on[0])],
            right_on: vec![ColumnIndex(on[1])],
            output: vec![
                JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(0),
                },
                JoinColumn {
                    side: Side::Right,
                    column: ColumnIndex(2),
                },
            ],
            filter,
            view: Vec::new(),
            reach: None,
        },
        &[left.1, right.1],
    )
}

/// The case vocabulary: every kind of this stage over `DEFINES` and over nodes, two selects keyed on one name column, and
/// the groupings and the graph kinds, then a select of the tree, last.
#[expect(
    clippy::too_many_lines,
    reason = "one program naming each kind of the stage once, in node order"
)]
pub fn case_program() -> Vec<Step> {
    let select = |name: &str| {
        step(
            Operation::Select {
                from: defines().0,
                comparisons: vec![equal(3, name)],
                kept: Vec::new(),
            },
            &[defines().1],
        )
    };
    let project = |at: u32, distinct: RepeatedRows| {
        step(
            Operation::Project {
                from: node(at).0,
                columns: vec![ColumnIndex(3)],
                distinct,
            },
            &[node(at).1],
        )
    };
    let pair = |at: [u32; 2]| [node(at[0]), node(at[1])];
    let set = |operation: fn(StepInput, StepInput) -> Operation, at: [u32; 2]| {
        let [left, right] = pair(at);
        step(operation(left.0, right.0), &[left.1, right.1])
    };
    vec![
        select("alpha"),
        select("beta"),
        project(0, RepeatedRows::Kept),
        project(1, RepeatedRows::Dropped),
        set(|left, right| Operation::Union { left, right }, [2, 3]),
        set(
            |left, right| Operation::Intersection { left, right },
            [2, 4],
        ),
        set(|left, right| Operation::Difference { left, right }, [4, 2]),
        step(
            Operation::Antijoin {
                left: defines().0,
                right: node(1).0,
                left_on: vec![ColumnIndex(3)],
                right_on: vec![ColumnIndex(3)],
            },
            &[defines().1, node(1).1],
        ),
        step(
            Operation::Extend {
                from: node(2).0,
                values: vec![Operand::Literal(QueryLiteral::Number(Integer(7)))],
            },
            &[node(2).1],
        ),
        join(node(0), defines(), [3, 3], Vec::new()),
        join(node(4), defines(), [0, 3], vec![equal(2, "other")]),
        step(
            Operation::GroupCount {
                from: defines().0,
                keys: vec![ColumnIndex(3)],
                distinct: None,
            },
            &[defines().1],
        ),
        step(
            Operation::GroupCount {
                from: defines().0,
                keys: Vec::new(),
                distinct: Some(ColumnIndex(2)),
            },
            &[defines().1],
        ),
        step(
            Operation::GroupDigest {
                from: defines().0,
                keys: vec![ColumnIndex(3)],
                over: ColumnIndex(2),
            },
            &[defines().1],
        ),
        step(
            Operation::Closure {
                edges: defines().0,
                keys: Vec::new(),
                source: ColumnIndex(0),
                destination: ColumnIndex(1),
            },
            &[defines().1],
        ),
        seeded_join(),
        step(
            Operation::Axis {
                from: node(0).0,
                edges: defines().0,
                domain: defines().0,
                direction: Direction::Parent,
                id: ColumnIndex(0),
                parent: ColumnIndex(0),
                child: ColumnIndex(1),
                domain_id: ColumnIndex(0),
                // Keyed edges admit by membership only: a domain filter is a tree walk's (main refuses it here).
                domain_filter: Vec::new(),
                domain_view: Vec::new(),
                output: Vec::new(),
            },
            &[node(0).1, defines().1, defines().1],
        ),
        step(
            // Over the name counts of node 11: each name an edge to itself, ordered by its count, a number.
            Operation::OrderedDfs {
                from: node(11).0,
                edges: node(11).0,
                root: ColumnIndex(0),
                source: ColumnIndex(0),
                destination: ColumnIndex(0),
                order: ColumnIndex(1),
                edge: ColumnIndex(1),
                root_keys: Vec::new(),
                edge_keys: Vec::new(),
            },
            &[node(11).1, node(11).1],
        ),
        step(
            Operation::Cycles {
                edges: defines().0,
                source: ColumnIndex(0),
                destination: ColumnIndex(1),
            },
            &[defines().1],
        ),
        step(
            Operation::Select {
                from: StepInput::Relation(Relation::SyntaxTree),
                comparisons: vec![equal(1, "word")],
                kept: Vec::new(),
            },
            &[PlanInput::Relation(Relation::SyntaxTree)],
        ),
    ]
}

/// A join of node 0's rows with a closure of `DEFINES` that the query planner fused into it: keyed on the closure's source.
fn seeded_join() -> Step {
    let Step { operation, inputs } = join(node(0), defines(), [0, 0], Vec::new());
    let Operation::Join {
        left,
        right,
        left_on,
        right_on,
        output,
        filter,
        view,
        ..
    } = operation
    else {
        unreachable!("join answers a join")
    };
    step(
        Operation::Join {
            left,
            right,
            left_on,
            right_on,
            output: output
                .into_iter()
                .map(|column| match column.side {
                    Side::Left => column,
                    Side::Right => JoinColumn {
                        side: Side::Right,
                        column: ColumnIndex(1),
                    },
                })
                .collect(),
            filter,
            view,
            reach: Some(ReachLookup {
                source: ColumnIndex(0),
                destination: ColumnIndex(1),
                keys: Vec::new(),
                columns: vec![ColumnIndex(0), ColumnIndex(1)],
            }),
        },
        &inputs,
    )
}

/// The ordered existence programs the rewrite and the run cases share.
#[cfg(test)]
pub(crate) mod ordered;
