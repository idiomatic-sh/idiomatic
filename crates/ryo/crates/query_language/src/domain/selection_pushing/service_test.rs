//! A filter of a union is made on both its inputs and a filter of a join on the input whose columns it tests, when every reader of the
//! union or the join is a filter; a union another step reads whole stays as it is.

use super::service::push_selections;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input as Reads, JoinColumn, LiteralText, Operand,
    PlanComparison, QueryLiteral, Side, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};

/// What a row asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// The only reader of a union is a filter.
    AFilteredUnion,
    /// The only reader of a join is a filter of a column of its right input.
    AFilteredJoin,
    /// A union is read by a filter and by a projection.
    AUnionReadWhole,
}

/// Whether the plan came back as the row expects it.
type Value = bool;
type Failure = std::convert::Infallible;
/// What a row asks of the service.
type Input = Scenario;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_filter_of_a_union_is_made_on_both_its_inputs"),
        word: Word::Succeeds,
        input: || Scenario::AFilteredUnion,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_filter_of_a_join_on_a_column_of_its_right_input_is_made_on_that_input"),
        word: Word::Succeeds,
        input: || Scenario::AFilteredJoin,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_union_another_step_reads_whole_stays_as_it_is"),
        word: Word::Succeeds,
        input: || Scenario::AUnionReadWhole,
        expect: Expect::Value(true),
    },
];

fn step(operation: Operation) -> PlanStep {
    PlanStep {
        operation,
        scope: StepScope::PerFile,
    }
}

fn select(from: Reads, comparisons: Vec<PlanComparison>) -> PlanStep {
    step(Operation::Select {
        from,
        comparisons,
        kept: Vec::new(),
    })
}

/// Column `column` equals the text "main".
fn named_main(column: u32) -> Vec<PlanComparison> {
    vec![PlanComparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(column)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText("main".into()))),
        origin: ComparisonOrigin::Unrecorded,
    }]
}

fn join(left: Reads, right: Reads) -> PlanStep {
    step(Operation::Join {
        left,
        right,
        left_on: vec![ColumnIndex(2)],
        right_on: vec![ColumnIndex(2)],
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
        reach: None,
    })
}

fn functions() -> PlanStep {
    select(Reads::Relation(Relation::Function), Vec::new())
}

fn comments() -> PlanStep {
    select(Reads::Relation(Relation::Comment), Vec::new())
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let union = step(Operation::Union {
        left: Reads::Step(StepId(0)),
        right: Reads::Step(StepId(1)),
    });
    let (before, emitted, after, emits) = match scenario {
        Scenario::AFilteredUnion => (
            vec![
                functions(),
                comments(),
                union,
                select(Reads::Step(StepId(2)), named_main(1)),
            ],
            StepId(3),
            vec![
                functions(),
                comments(),
                select(Reads::Step(StepId(0)), named_main(1)),
                select(Reads::Step(StepId(1)), named_main(1)),
                step(Operation::Union {
                    left: Reads::Step(StepId(2)),
                    right: Reads::Step(StepId(3)),
                }),
            ],
            StepId(4),
        ),
        Scenario::AFilteredJoin => (
            vec![
                functions(),
                comments(),
                join(Reads::Step(StepId(0)), Reads::Step(StepId(1))),
                select(Reads::Step(StepId(2)), named_main(1)),
            ],
            StepId(3),
            vec![
                functions(),
                comments(),
                select(Reads::Step(StepId(1)), named_main(1)),
                join(Reads::Step(StepId(0)), Reads::Step(StepId(2))),
            ],
            StepId(3),
        ),
        Scenario::AUnionReadWhole => {
            let steps = vec![
                functions(),
                comments(),
                union,
                select(Reads::Step(StepId(2)), named_main(1)),
                step(Operation::Join {
                    left: Reads::Step(StepId(3)),
                    right: Reads::Step(StepId(2)),
                    left_on: vec![ColumnIndex(0)],
                    right_on: vec![ColumnIndex(0)],
                    output: Vec::new(),
                    filter: Vec::new(),
                    view: Vec::new(),
                    reach: None,
                }),
            ];
            (steps.clone(), StepId(4), steps, StepId(4))
        }
    };
    let planned = push_selections(before, emitted);
    Ok(planned.steps == after && planned.emitted == emits)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
