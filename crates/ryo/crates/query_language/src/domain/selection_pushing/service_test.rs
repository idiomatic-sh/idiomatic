//! A filter of a join keyed on the file alone is made on the input whose columns it tests, and a filter of a union of such a join on
//! both its inputs, when every reader of the union or the join is a filter; a union another step reads whole stays as it is.

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
    /// The only reader of a union of a join keyed on the file is a filter.
    FilteredUnion,
    /// The only reader of a join keyed on the file is a filter of a column of its right input.
    FilteredJoin,
    /// A union is read by a filter and by a join.
    UnionReadWhole,
}

/// Whether the plan came back as the row expects it.
type Value = bool;
type Failure = std::convert::Infallible;
/// What a row asks of the service.
type Input = Scenario;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "a_filter_of_a_union_of_a_join_keyed_on_the_file_is_made_on_both_its_inputs_and_into_the_join",
        ),
        word: Word::Succeeds,
        input: || Scenario::FilteredUnion,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName(
            "a_filter_of_a_join_keyed_on_the_file_on_a_column_of_its_right_input_is_made_on_that_input",
        ),
        word: Word::Succeeds,
        input: || Scenario::FilteredJoin,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_union_another_step_reads_whole_stays_as_it_is"),
        word: Word::Succeeds,
        input: || Scenario::UnionReadWhole,
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
        left: Reads::Step(StepId(2)),
        right: Reads::Step(StepId(1)),
    });
    let (before, emitted, after, emits) = match scenario {
        Scenario::FilteredUnion => (
            vec![
                functions(),
                comments(),
                join(Reads::Step(StepId(0)), Reads::Step(StepId(1))),
                step(Operation::Union {
                    left: Reads::Step(StepId(2)),
                    right: Reads::Step(StepId(1)),
                }),
                select(Reads::Step(StepId(3)), named_main(1)),
            ],
            StepId(4),
            vec![
                functions(),
                comments(),
                select(Reads::Step(StepId(1)), named_main(1)),
                join(Reads::Step(StepId(0)), Reads::Step(StepId(2))),
                select(Reads::Step(StepId(1)), named_main(1)),
                step(Operation::Union {
                    left: Reads::Step(StepId(3)),
                    right: Reads::Step(StepId(4)),
                }),
            ],
            StepId(5),
        ),
        Scenario::FilteredJoin => (
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
        Scenario::UnionReadWhole => {
            let steps = vec![
                functions(),
                comments(),
                join(Reads::Step(StepId(0)), Reads::Step(StepId(1))),
                union,
                select(Reads::Step(StepId(3)), named_main(1)),
                step(Operation::Join {
                    left: Reads::Step(StepId(4)),
                    right: Reads::Step(StepId(3)),
                    left_on: vec![ColumnIndex(0)],
                    right_on: vec![ColumnIndex(0)],
                    output: Vec::new(),
                    filter: Vec::new(),
                    view: Vec::new(),
                    reach: None,
                }),
            ];
            (steps.clone(), StepId(5), steps, StepId(5))
        }
    };
    let planned = push_selections(before, emitted);
    Ok(planned.steps == after && planned.emitted == emits)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
