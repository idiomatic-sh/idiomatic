//! Filters on the columns a capture was given are made before it, and no other filter moves.

use super::service::push_below_captures;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    CapturePattern, ColumnIndex, Input as Reads, LiteralText, Operand, PlanComparison, QueryLiteral,
    StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};

/// What a row asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// A filter on a column the capture read.
    OnAColumnItRead,
    /// A filter on a column the capture appended.
    OnAColumnItAppended,
}

/// Whether the plan came back changed.
type Value = bool;
type Failure = std::convert::Infallible;
/// What a row asks of the service.
type Input = Scenario;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_filter_on_a_column_the_capture_read_is_made_before_it"),
        word: Word::Succeeds,
        input: || Scenario::OnAColumnItRead,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_filter_on_a_column_the_capture_appended_stays_after_it"),
        word: Word::Succeeds,
        input: || Scenario::OnAColumnItAppended,
        expect: Expect::Value(false),
    },
];

fn step(operation: Operation) -> PlanStep {
    PlanStep {
        operation,
        scope: StepScope::PerFile,
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let asked = match scenario {
        Scenario::OnAColumnItRead => ColumnIndex(0),
        Scenario::OnAColumnItAppended => ColumnIndex(99),
    };
    let steps = vec![
        step(Operation::Select {
            from: Reads::Relation(Relation::Function),
            comparisons: Vec::new(),
            kept: Vec::new(),
        }),
        step(Operation::Capture {
            from: Reads::Step(StepId(0)),
            path: ColumnIndex(1),
            pattern: CapturePattern("{*dir}/x.rs".into()),
        }),
        step(Operation::Select {
            from: Reads::Step(StepId(1)),
            comparisons: vec![PlanComparison {
                test: Test::Equal,
                left: Operand::Column(asked),
                right: Operand::Literal(QueryLiteral::Text(LiteralText("main".into()))),
                origin: None,
            }],
            kept: Vec::new(),
        }),
    ];
    let planned = push_below_captures(steps.clone(), StepId(2));
    Ok(planned.steps != steps)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
