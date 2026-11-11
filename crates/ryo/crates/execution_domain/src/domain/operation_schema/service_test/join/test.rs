//! Missing RHS is a schema/input refusal, independent of any owner or source.
use std::borrow::Cow;

use super::fixtures::NUMBER;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operation_schema::types::InputShape;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

/// The inputs the join of `entry` and `sequence_element` on their first columns, answering the left's first column, is given.
type Input = &'static [InputShape<'static>];
/// The kinds of the columns the join answers.
type Value = Cow<'static, [CellKind]>;
type Failure = OperatingError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_join_with_its_two_inputs_answers_its_output_columns"),
        word: Word::Succeeds,
        input: || &[NUMBER, NUMBER],
        expect: Expect::Value(Cow::Borrowed(&[CellKind::Number])),
    },
    Case {
        name: CaseName("a_join_without_its_second_input_reports_position_two"),
        word: Word::Refuses,
        input: || &[NUMBER],
        expect: Expect::Error(OperatingError::MissingInput { position: 2 }),
    },
];

/// Runs the join over the row's inputs.
fn subject(input: Input) -> Result<Value, Failure> {
    super::fixtures::subject(input)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
