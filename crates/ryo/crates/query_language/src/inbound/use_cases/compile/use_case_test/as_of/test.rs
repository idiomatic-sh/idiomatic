//! The `as-of` call, from the compile inbound API: the step it compiles to, where it runs, and what it refuses.

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

/// Each event with the events of its name and file latest before it.
const BEFORE: &str = r#"let events = select scope, name, file, ordinal from "binding-event"; let a = as-of(left=events, right=events, left-on=[name, file], right-on=[name, file], left-order="ordinal", right-order="ordinal", bound=before, output={name: [left, name], file: [left, file], point: [left, ordinal], earlier: [right, ordinal]}); emit a subject file key(name, point, earlier);"#;
/// The same with the other side and the inclusive bound.
const AT_OR_AFTER: &str = r#"let events = select scope, name, file, ordinal from "binding-event"; let a = as-of(left=events, right=events, left-on=[name, file], right-on=[name, file], left-order="ordinal", right-order="ordinal", bound=at-or-after, output={name: [left, name], file: [left, file], point: [left, ordinal], later: [right, ordinal]}); emit a subject file key(name, point, later);"#;
/// No key: every row is in one group, so the step runs after the files.
const ONE_GROUP: &str = r#"let events = select scope, name, file, ordinal from "binding-event"; let a = as-of(left=events, right=events, left-on=[], right-on=[], left-order="ordinal", right-order="ordinal", bound=after, output={file: [left, file], point: [left, ordinal], later: [right, ordinal]}); emit a subject file key(point, later);"#;
/// An order column of text.
const TEXT_ORDER: &str = r#"let events = select scope, name, file, ordinal from "binding-event"; let a = as-of(left=events, right=events, left-on=[file], right-on=[file], left-order="name", right-order="ordinal", bound=before, output={file: [left, file], point: [left, ordinal]}); emit a subject file key(point);"#;
/// A bound that is none of the four.
const NO_SUCH_BOUND: &str = r#"let events = select scope, name, file, ordinal from "binding-event"; let a = as-of(left=events, right=events, left-on=[file], right-on=[file], left-order="ordinal", right-order="ordinal", bound=nearest, output={file: [left, file], point: [left, ordinal]}); emit a subject file key(point);"#;
/// No bound at all.
const NO_BOUND: &str = r#"let events = select scope, name, file, ordinal from "binding-event"; let a = as-of(left=events, right=events, left-on=[file], right-on=[file], left-order="ordinal", right-order="ordinal", output={file: [left, file], point: [left, ordinal]}); emit a subject file key(point);"#;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("as_of_compiles_to_one_step_with_its_bound"),
        word: Word::Succeeds,
        input: || BEFORE,
        expect: Expect::Value(Cow::Borrowed(
            "AsOf { left: Step(StepId(0)), right: Step(StepId(0)), left_on: [ColumnIndex(1), ColumnIndex(2)], right_on: [ColumnIndex(1), ColumnIndex(2)], left_order: ColumnIndex(3), right_order: ColumnIndex(3), bound: Before, output: [JoinColumn { side: Left, column: ColumnIndex(1) }, JoinColumn { side: Left, column: ColumnIndex(2) }, JoinColumn { side: Left, column: ColumnIndex(3) }, JoinColumn { side: Right, column: ColumnIndex(3) }] }",
        )),
    },
    Case {
        name: CaseName("each_of_the_four_bounds_is_a_word_of_the_call"),
        word: Word::Succeeds,
        input: || AT_OR_AFTER,
        expect: Expect::Value(Cow::Borrowed(
            "AsOf { left: Step(StepId(0)), right: Step(StepId(0)), left_on: [ColumnIndex(1), ColumnIndex(2)], right_on: [ColumnIndex(1), ColumnIndex(2)], left_order: ColumnIndex(3), right_order: ColumnIndex(3), bound: AtOrAfter, output: [JoinColumn { side: Left, column: ColumnIndex(1) }, JoinColumn { side: Left, column: ColumnIndex(2) }, JoinColumn { side: Left, column: ColumnIndex(3) }, JoinColumn { side: Right, column: ColumnIndex(3) }] }",
        )),
    },
    Case {
        name: CaseName("as_of_with_no_key_is_one_group_after_the_files"),
        word: Word::Succeeds,
        input: || ONE_GROUP,
        expect: Expect::Value(Cow::Borrowed(
            "AsOf { left: Step(StepId(0)), right: Step(StepId(0)), left_on: [], right_on: [], left_order: ColumnIndex(3), right_order: ColumnIndex(3), bound: After, output: [JoinColumn { side: Left, column: ColumnIndex(2) }, JoinColumn { side: Left, column: ColumnIndex(3) }, JoinColumn { side: Right, column: ColumnIndex(3) }] }*",
        )),
    },
    Case {
        name: CaseName("an_order_column_that_is_not_a_number_is_refused"),
        word: Word::Refuses,
        input: || TEXT_ORDER,
        expect: Expect::Error(Cow::Borrowed(
            "`as-of` orders by a number, and its order column `name` is not one (from `<query>` at byte 69)",
        )),
    },
    Case {
        name: CaseName("a_bound_that_is_not_one_of_the_four_is_refused"),
        word: Word::Refuses,
        input: || NO_SUCH_BOUND,
        expect: Expect::Error(Cow::Borrowed(
            "`as-of` cannot use its argument `bound`: expected before, at-or-before, after or at-or-after (from `<query>` at byte 69)",
        )),
    },
    Case {
        name: CaseName("as_of_without_a_bound_is_refused"),
        word: Word::Refuses,
        input: || NO_BOUND,
        expect: Expect::Error(Cow::Borrowed(
            "`as-of` needs the argument `bound` (from `<query>` at byte 69)",
        )),
    },
];

fn subject(text: Input) -> Result<Value, Failure> {
    compiled_steps(text, &[OperationKind::AsOf])
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
