//! The parts the lowering's fixture plans are built from: a plan of steps, the words of a file, and the steps over them.

use ryo_relations::domain::relations::types::{Integer, Relation};
use ryo_values::domain::plan_orders::types::OrderBound;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Direction, Heading, Input, JoinColumn, LiteralText, Operand,
    PlanComparison, QueryLiteral, RepeatedRows, Side, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::query_plans::types::{Emit, Plan};

/// A step that keeps every row of `from`.
pub(super) fn kept(from: Input) -> PlanStep {
    PlanStep {
        written: Default::default(),
        operation: Operation::Select {
            from,
            comparisons: Vec::new(),
            kept: Vec::new(),
        },
        scope: StepScope::PerFile,
    }
}

/// A plan of `steps` emitting the last one's rows.
pub(super) fn plan(steps: Vec<PlanStep>) -> Plan {
    let last = u32::try_from(steps.len().saturating_sub(1)).unwrap_or(u32::MAX);
    Plan {
        observation_inputs: Vec::new(),
        parameters: Vec::new(),
        steps,
        comparison_origins: Vec::new(),
        observations: Vec::new(),
        emit: Emit {
            step: StepId(last),
            subject: ColumnIndex(0),
            key: vec![ColumnIndex(0)],
            headings: vec![Heading("node".into())],
        },
        frontier: Vec::new(),
    }
}

/// A per-file step of `operation`.
pub(super) fn step(operation: Operation) -> PlanStep {
    PlanStep {
        written: Default::default(),
        operation,
        scope: StepScope::PerFile,
    }
}

/// The column at `place`.
pub(super) const fn at(place: u32) -> ColumnIndex {
    ColumnIndex(place)
}

/// A column of one side of a join.
pub(super) const fn of(side: Side, place: u32) -> JoinColumn {
    JoinColumn {
        side,
        column: ColumnIndex(place),
    }
}

/// A plain join of two inputs on equal keys.
pub(super) fn joined(
    left: Input,
    right: Input,
    on: (&[u32], &[u32]),
    output: Vec<JoinColumn>,
) -> PlanStep {
    step(Operation::Join {
        left,
        right,
        left_on: on.0.iter().copied().map(ColumnIndex).collect(),
        right_on: on.1.iter().copied().map(ColumnIndex).collect(),
        output,
        filter: Vec::new(),
        view: Vec::new(),
        reach: None,
    })
}

/// The rows of `left` that no row of step 0 matches on the columns `on` names of each: an antijoin whose right side is step 0.
pub(super) fn without(left: Input, on: (&[u32], &[u32])) -> PlanStep {
    step(Operation::Antijoin {
        left,
        right: Input::Step(StepId(0)),
        left_on: on.0.iter().copied().map(ColumnIndex).collect(),
        right_on: on.1.iter().copied().map(ColumnIndex).collect(),
    })
}

/// The words of a file, one row for each child of its sentence: parent, child, ordinal, text, file. Step 0 of the scenes below.
pub(super) fn words() -> PlanStep {
    joined(
        Input::Relation(Relation::SyntaxChild),
        Input::Relation(Relation::SyntaxTree),
        (&[1], &[0]),
        vec![
            of(Side::Left, 0),
            of(Side::Left, 1),
            of(Side::Left, 2),
            of(Side::Right, 2),
            of(Side::Left, 4),
        ],
    )
}

/// The words (step 0) whose text is `text`, keeping `kept` of their columns; none named keeps every column.
pub(super) fn words_written(text: &str, kept: &[u32]) -> PlanStep {
    step(Operation::Select {
        from: Input::Step(StepId(0)),
        comparisons: vec![PlanComparison {
            test: Test::Equal,
            left: Operand::Column(at(3)),
            right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
            origin: ComparisonOrigin::Unrecorded,
        }],
        kept: kept.iter().copied().map(ColumnIndex).collect(),
    })
}

/// An as-of of two steps of words by ordinal, answering the left word, the right word and the file.
pub(super) fn as_of(left: (u32, &[u32]), right: (u32, &[u32], u32), bound: OrderBound) -> PlanStep {
    step(Operation::AsOf {
        left: Input::Step(StepId(left.0)),
        right: Input::Step(StepId(right.0)),
        left_on: left.1.iter().copied().map(ColumnIndex).collect(),
        right_on: right.1.iter().copied().map(ColumnIndex).collect(),
        left_order: at(2),
        right_order: at(right.2),
        bound,
        output: vec![of(Side::Left, 1), of(Side::Right, 1), of(Side::Left, 4)],
    })
}

/// A comparison of a column with a number the query wrote.
pub(super) fn compared(test: Test, column: u32, number: i64) -> PlanComparison {
    PlanComparison {
        test,
        left: Operand::Column(at(column)),
        right: Operand::Literal(QueryLiteral::Number(Integer(number))),
        origin: ComparisonOrigin::Unrecorded,
    }
}

/// The words (step 0) that `comparison` holds of, keeping every column.
pub(super) fn words_where(comparison: PlanComparison) -> PlanStep {
    step(Operation::Select {
        from: Input::Step(StepId(0)),
        comparisons: vec![comparison],
        kept: Vec::new(),
    })
}

/// A projection of a step onto `columns`, each row kept.
pub(super) fn projected(from: u32, columns: &[u32]) -> PlanStep {
    step(Operation::Project {
        from: Input::Step(StepId(from)),
        columns: columns.iter().copied().map(ColumnIndex).collect(),
        distinct: RepeatedRows::Kept,
    })
}

/// A nearest-ancestor axis from the rows of step 0 over the given edges, its domain step 0 too.
pub(super) fn nearest_step(edges: Input) -> PlanStep {
    axis(
        edges,
        Direction::NearestAncestor,
        Input::Step(StepId(0)),
        Vec::new(),
    )
}

/// An axis from the rows of step 0 over the given edges, its domain read through the given view.
pub(super) fn axis(
    edges: Input,
    direction: Direction,
    domain: Input,
    domain_view: Vec<ColumnIndex>,
) -> PlanStep {
    step(Operation::Axis {
        from: Input::Step(StepId(0)),
        edges,
        domain,
        direction,
        id: at(0),
        parent: at(0),
        child: at(1),
        domain_id: at(0),
        domain_filter: Vec::new(),
        domain_view,
        output: Vec::new(),
    })
}

/// The elements of the syntax tree, each beside its nearest ancestor among them, climbed over the given edges.
pub(super) fn nearest(edges: Input) -> Plan {
    plan(vec![
        kept(Input::Relation(Relation::SyntaxTree)),
        nearest_step(edges),
    ])
}
