//! The rows a file hands over are cut to the columns the steps after the files ask, and no further, whatever step made them.

use super::service::{cut_handover, cut_relations};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as Reads, StepId, StepScope};
use ryo_values::domain::plans::types::{Operation, PlanStep};

/// What a row asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// A step after the files asks of one column of a relation.
    OneColumnAsked,
    /// A step after the files asks of every column of a relation.
    EveryColumnAsked,
    /// A step after the files asks of two of the three columns a join in each file makes.
    TwoOfAJoinsColumnsAsked,
    /// A step after the files asks of every column a join in each file makes.
    EveryColumnOfAJoinAsked,
}

/// Whether the plan came back with a step that cuts the relation.
type Value = bool;
type Failure = std::convert::Infallible;
/// What a row asks of the service.
type Input = Scenario;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_relation_asked_for_one_column_is_cut_to_it"),
        word: Word::Succeeds,
        input: || Scenario::OneColumnAsked,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_relation_asked_for_every_column_is_left_as_it_is"),
        word: Word::Succeeds,
        input: || Scenario::EveryColumnAsked,
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("a_join_in_each_file_asked_for_fewer_columns_than_it_makes_is_cut_to_them"),
        word: Word::Succeeds,
        input: || Scenario::TwoOfAJoinsColumnsAsked,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_join_in_each_file_asked_for_every_column_it_makes_is_left_as_it_is"),
        word: Word::Succeeds,
        input: || Scenario::EveryColumnOfAJoinAsked,
        expect: Expect::Value(false),
    },
];

/// The functions of each file joined in it with its comments on the file, keeping three columns, and a step after the files that
/// asks of the columns `asked` of that join: whether the plan came back with a step that cuts the join.
fn join_asked(asked: Vec<ColumnIndex>) -> Value {
    use ryo_values::domain::plan_parts::types::{JoinColumn, Side};
    let column = |side, place| JoinColumn {
        side,
        column: ColumnIndex(place),
    };
    let steps = vec![
        PlanStep {
            operation: Operation::Join {
                left: Reads::Relation(Relation::Function),
                right: Reads::Relation(Relation::Comment),
                left_on: vec![ColumnIndex(2)],
                right_on: vec![ColumnIndex(2)],
                output: vec![
                    column(Side::Left, 0),
                    column(Side::Left, 1),
                    column(Side::Right, 0),
                ],
                filter: Vec::new(),
                view: Vec::new(),
                reach: None,
            },
            scope: StepScope::PerFile,
        },
        PlanStep {
            operation: Operation::Select {
                from: Reads::Step(StepId(0)),
                comparisons: Vec::new(),
                kept: asked,
            },
            scope: StepScope::CrossFile,
        },
    ];
    cut_handover(steps.clone(), StepId(1)).steps.len() > steps.len()
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let kept = match scenario {
        Scenario::OneColumnAsked => vec![ColumnIndex(0)],
        Scenario::EveryColumnAsked => Vec::new(),
        Scenario::TwoOfAJoinsColumnsAsked => {
            return Ok(join_asked(vec![ColumnIndex(0), ColumnIndex(2)]));
        }
        Scenario::EveryColumnOfAJoinAsked => {
            return Ok(join_asked(vec![
                ColumnIndex(0),
                ColumnIndex(1),
                ColumnIndex(2),
            ]));
        }
    };
    let steps = vec![PlanStep {
        operation: Operation::Select {
            from: Reads::Relation(Relation::Function),
            comparisons: Vec::new(),
            kept,
        },
        scope: StepScope::CrossFile,
    }];
    let planned = cut_relations(steps.clone(), StepId(0));
    Ok(planned.steps.len() > steps.len())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
