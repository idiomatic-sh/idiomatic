//! The `rewriting` service's cases, on plans written by hand.

use ryo_relations::domain::relations::types::Relation;

use super::service::same_sets;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as Reads;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;
use ryo_values::domain::plan_parts::types::StepScope;

/// What a row asks of the service.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scenario {
    /// The two antijoins of the texts, each way, are the idiom.
    TheIdiom,
    /// The second direction joins the texts of the same function as the first.
    BothDirectionsJoinTheFirstFunction,
    /// The second direction asks about the wrong columns.
    TheSecondDirectionAsksTheWrongColumns,
}

/// A number the answer is.
type Value = usize;
type Failure = ();
/// What a row asks of the service.
type Input = Scenario;

fn column(index: u32) -> ColumnIndex {
    ColumnIndex(index)
}

fn step(operation: Operation) -> PlanStep {
    PlanStep {
        operation,
        scope: StepScope::CrossFile,
    }
}

/// The steps of the idiom: the pairs, the texts, and the two directions with the mismatches they find.
fn idiom(second_join_on: u32, second_asks: [u32; 2]) -> Vec<PlanStep> {
    let rows = [
        JoinColumn {
            side: Side::Left,
            column: column(0),
        },
        JoinColumn {
            side: Side::Left,
            column: column(1),
        },
        JoinColumn {
            side: Side::Right,
            column: column(1),
        },
    ];
    vec![
        step(Operation::Select {
            from: Reads::Relation(Relation::Function),
            comparisons: Vec::new(),
            kept: Vec::new(),
        }),
        step(Operation::Select {
            from: Reads::Relation(Relation::SyntaxTree),
            comparisons: Vec::new(),
            kept: Vec::new(),
        }),
        step(Operation::Join {
            left: Reads::Step(StepId(0)),
            right: Reads::Step(StepId(1)),
            left_on: vec![column(0)],
            right_on: vec![column(0)],
            output: rows.to_vec(),
            filter: Vec::new(),
            view: Vec::new(),

            reach: None,
        }),
        step(Operation::Antijoin {
            left: Reads::Step(StepId(2)),
            right: Reads::Step(StepId(1)),
            left_on: vec![column(1), column(2)],
            right_on: vec![column(0), column(1)],
        }),
        step(Operation::Project {
            from: Reads::Step(StepId(3)),
            columns: vec![column(0), column(1)],
            distinct: RepeatedRows::Dropped,
        }),
        step(Operation::Join {
            left: Reads::Step(StepId(0)),
            right: Reads::Step(StepId(1)),
            left_on: vec![column(second_join_on)],
            right_on: vec![column(0)],
            output: rows.to_vec(),
            filter: Vec::new(),
            view: Vec::new(),

            reach: None,
        }),
        step(Operation::Antijoin {
            left: Reads::Step(StepId(5)),
            right: Reads::Step(StepId(1)),
            left_on: second_asks.iter().map(|each| column(*each)).collect(),
            right_on: vec![column(0), column(1)],
        }),
        step(Operation::Project {
            from: Reads::Step(StepId(6)),
            columns: vec![column(0), column(1)],
            distinct: RepeatedRows::Dropped,
        }),
        step(Operation::Union {
            left: Reads::Step(StepId(4)),
            right: Reads::Step(StepId(7)),
        }),
    ]
}

fn found(steps: &[PlanStep]) -> usize {
    usize::from(
        same_sets(
            steps,
            Reads::Step(StepId(0)),
            Reads::Step(StepId(8)),
            &[column(0), column(1)],
            &[column(0), column(1)],
        )
        .is_some_and(|idiom| {
            idiom.texts == StepId(1)
                && idiom.function == column(0)
                && idiom.other == column(1)
                && idiom.owner == column(0)
                && idiom.text == column(1)
        }),
    )
}

fn subject(scenario: Input) -> Value {
    match scenario {
        Scenario::TheIdiom => found(&idiom(1, [0, 2])),
        Scenario::BothDirectionsJoinTheFirstFunction => found(&idiom(0, [0, 2])),
        Scenario::TheSecondDirectionAsksTheWrongColumns => found(&idiom(1, [1, 2])),
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("two_antijoins_of_the_texts_each_way_are_the_idiom"),
        word: Word::Succeeds,
        input: || Scenario::TheIdiom,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("two_directions_over_the_same_function_are_not"),
        word: Word::Succeeds,
        input: || Scenario::BothDirectionsJoinTheFirstFunction,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_direction_asking_the_wrong_columns_is_not"),
        word: Word::Succeeds,
        input: || Scenario::TheSecondDirectionAsksTheWrongColumns,
        expect: Expect::Value(0),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
