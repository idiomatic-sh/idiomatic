//! Test support of the `physical_plan` concept: the steps of a case's program, and the case vocabulary the catalogs and the
//! rewrites are checked over.

use crate::domain::physical_plan::types::{PlanInput, PlanNodeId};
use ryo_relations::domain::relations::types::{Integer, Relation};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input as StepInput, JoinColumn, LiteralText, Operand,
    PlanComparison, QueryLiteral, RepeatedRows, Side, StepId, Test,
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
/// the groupings, then a select of the tree, last.
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
            Operation::Select {
                from: StepInput::Relation(Relation::SyntaxTree),
                comparisons: vec![equal(1, "word")],
                kept: Vec::new(),
            },
            &[PlanInput::Relation(Relation::SyntaxTree)],
        ),
    ]
}

/// The case program followed by one node of each text kind, each reading Defines: the program the catalogs of the served
/// stages are checked over.
#[must_use]
pub fn text_program() -> Vec<Step> {
    use ryo_values::domain::plan_parts::types::{CapturePattern, ParameterId};
    use ryo_values::domain::query_operators::types::{LineOutput, MaxLine, TabWidth, TextOutput};
    let from = || defines().0;
    let read = |operation: Operation| step(operation, &[defines().1]);
    let literal = |text: &str| LiteralText(text.into());
    let mut steps = case_program();
    steps.extend([
        read(Operation::Capture {
            from: from(),
            path: ColumnIndex(3),
            pattern: CapturePattern("{x}".into()),
        }),
        read(Operation::Rank {
            from: from(),
            column: ColumnIndex(3),
            sets: ParameterId(0),
        }),
        read(Operation::Replace {
            from: from(),
            text: ColumnIndex(3),
            pattern: literal("a"),
            replacement: literal("b"),
        }),
        step(
            Operation::OrderedMatch {
                left: from(),
                right: from(),
                left_on: vec![ColumnIndex(3)],
                right_on: vec![ColumnIndex(3)],
                left_order: ColumnIndex(0),
                right_order: ColumnIndex(0),
                output: Vec::new(),
            },
            &[defines().1, defines().1],
        ),
        read(Operation::Words {
            from: from(),
            text: ColumnIndex(3),
        }),
        read(Operation::SpanBetween {
            from: from(),
            start: ColumnIndex(6),
            end: ColumnIndex(6),
        }),
        read(Operation::SourceMatches {
            from: from(),
            span: ColumnIndex(6),
            base_line: None,
            pattern: literal("a"),
            capture: None,
            capture_text: TextOutput::Omitted,
            capture_line: LineOutput::Omitted,
        }),
        read(Operation::RenderSyntax {
            from: from(),
            node: ColumnIndex(0),
            context: ryo_language::domain::semantics::types::RenderContext::PositionalArgument,
        }),
        read(Operation::FitsRendered {
            from: from(),
            texts: vec![ColumnIndex(3)],
            literals: vec![literal(""), literal("")],
            span: ColumnIndex(6),
            max_line: MaxLine(80),
            tab_width: TabWidth(4),
        }),
        read(Operation::SpanBounds {
            from: from(),
            span: ColumnIndex(6),
        }),
    ]);
    steps
}

/// Defines rows of file 0, one for each `(name, span)`, each its own node by its place; the kind and refinement cells are the
/// name too.
#[must_use]
pub fn defines_rows(
    rows: &[(
        ryo_relations::domain::relations::types::Value,
        ryo_relations::domain::relations::types::Span,
    )],
) -> ryo_values::domain::rows::types::Table {
    use ryo_relations::domain::relations::types::{FileId, Position, Value};
    let mut cells = Vec::with_capacity(rows.len() * 7);
    for ((name, span), at) in rows.iter().zip(0..) {
        let node = Value::Node(ryo_relations::domain::relations::fixtures::node_id(
            FileId(0),
            Position(at),
        ));
        cells.extend([
            node,
            *name,
            *name,
            *name,
            node,
            Value::File(FileId(0)),
            Value::Span(*span),
        ]);
    }
    ryo_values::domain::rows::types::Table {
        kinds: ryo_relations::domain::relations::fixtures::kinds(DEFINES).to_vec(),
        cells,
        count: ryo_values::domain::rows::types::RowCount(rows.len()),
    }
}

/// One node reading Defines.
#[must_use]
pub fn over_defines(operation: Operation) -> Step {
    step(operation, &[defines().1])
}

/// One node reading the earlier nodes `at`, in order.
#[must_use]
pub fn over_nodes(operation: Operation, at: &[u32]) -> Step {
    let inputs: Vec<PlanInput> = at.iter().map(|at| node(*at).1).collect();
    step(operation, &inputs)
}
