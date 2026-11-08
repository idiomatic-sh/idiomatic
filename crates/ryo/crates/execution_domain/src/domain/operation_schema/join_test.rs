//! Missing RHS is a schema/input refusal, independent of any owner or source.
use super::{service::schema_of, types::InputShape};
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input as Source, JoinColumn, Side},
    plans::types::Operation,
};

/// Which inputs the join is given.
#[derive(Clone, Copy)]
enum Scenario {
    Both,
    FirstOnly,
}
type Input = Scenario;
/// How many columns the join answers.
type Value = usize;
type Failure = OperatingError;

fn subject(scenario: Input) -> Result<Value, Failure> {
    let operation = Operation::Join {
        left: Source::Relation(Relation::Entry),
        right: Source::Relation(Relation::SequenceElement),
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(0)],
        output: vec![JoinColumn {
            side: Side::Left,
            column: ColumnIndex(0),
        }],
        filter: vec![],
        view: vec![],
        reach: None,
    };
    let first = InputShape {
        kinds: &[CellKind::Number],
        tree: None,
    };
    let inputs = match scenario {
        Scenario::Both => vec![first, first],
        Scenario::FirstOnly => vec![first],
    };
    schema_of(&operation, &inputs, &[]).map(|kinds| kinds.len())
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_join_with_its_two_inputs_answers_its_output_columns"),
        word: Word::Succeeds,
        input: || Scenario::Both,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_join_without_its_second_input_reports_position_two"),
        word: Word::Refuses,
        input: || Scenario::FirstOnly,
        expect: Expect::Error(OperatingError::MissingInput { position: 2 }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
