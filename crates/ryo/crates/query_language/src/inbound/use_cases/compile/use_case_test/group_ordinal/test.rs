//! The `group-ordinal` call, from the compile inbound API: the step it compiles to, the column it appends, and what it refuses.

use super::super::compiled_steps;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::query_operators::types::OperationKind;
use std::borrow::Cow;

/// The query text a row gives.
type Input = &'static str;
/// The plan's steps of the kind this part compiles, as the plan holds them, each starred when it runs after the files.
type Value = Cow<'static, str>;
/// What the compile use case refuses a row with, as its caller reads it.
type Failure = Cow<'static, str>;

/// Each event's place among the events of its name and file; the place is a column the next step reads.
const PLACED: &str = r#"let events = select scope, name, file, ordinal from "binding-event"; let placed = group-ordinal(from=events, keys=[name, file], order="ordinal", output="place"); let a = select name, file, place from placed where place = 0; emit a subject file key(name, place);"#;
/// A place named as a column the input already has.
const TAKEN_NAME: &str = r#"let events = select scope, name, file, ordinal from "binding-event"; let a = group-ordinal(from=events, keys=[name, file], order="ordinal", output="name"); emit a subject file key(name);"#;
/// An order column of text.
const TEXT_ORDER: &str = r#"let events = select scope, name, file, ordinal from "binding-event"; let a = group-ordinal(from=events, keys=[file], order="name", output="place"); emit a subject file key(place);"#;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("group_ordinal_appends_a_number_the_next_step_reads"),
        word: Word::Succeeds,
        input: || PLACED,
        expect: Expect::Value(Cow::Borrowed(
            "GroupOrdinal { from: Step(StepId(0)), keys: [ColumnIndex(1), ColumnIndex(2)], order: ColumnIndex(3) }; Select { from: Step(StepId(1)), comparisons: [PlanComparison { test: Equal, left: Column(ColumnIndex(4)), right: Literal(Number(Integer(0))), origin: Recorded(ComparisonOriginId(0)) }], kept: [] }",
        )),
    },
    Case {
        name: CaseName("a_place_named_as_a_column_of_the_input_is_refused"),
        word: Word::Refuses,
        input: || TAKEN_NAME,
        expect: Expect::Error(Cow::Borrowed(
            "output column `name` already exists (from `<query>` at byte 69)",
        )),
    },
    Case {
        name: CaseName("an_order_column_that_is_not_a_number_is_refused"),
        word: Word::Refuses,
        input: || TEXT_ORDER,
        expect: Expect::Error(Cow::Borrowed(
            "`group-ordinal` orders by a number, and its order column `name` is not one (from `<query>` at byte 69)",
        )),
    },
];

fn subject(text: Input) -> Result<Value, Failure> {
    compiled_steps(text, &[OperationKind::GroupOrdinal, OperationKind::Select])
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
