//! The `group-min` and `group-max` calls, from the compile inbound API: the step each compiles to, where it runs, and what it refuses.

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

/// The last event of each name in each file.
const GROUP_MAX: &str = r#"let events = select scope, name, file, ordinal from "binding-event"; let a = group-max(from=events, keys=[name, file], order="ordinal"); emit a subject file key(name, ordinal);"#;
/// The first event of each name in each file.
const GROUP_MIN: &str = r#"let events = select scope, name, file, ordinal from "binding-event"; let a = group-min(from=events, keys=[name, file], order="ordinal"); emit a subject file key(name, ordinal);"#;
/// No key: one group of every file's rows, so the step runs after the files.
const ONE_GROUP: &str = r#"let events = select scope, name, file, ordinal from "binding-event"; let a = group-max(from=events, keys=[], order="ordinal"); emit a subject file key(name, ordinal);"#;
/// An order column of text.
const TEXT_ORDER: &str = r#"let events = select scope, name, file, ordinal from "binding-event"; let a = group-min(from=events, keys=[file], order="name"); emit a subject file key(name);"#;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("group_max_keeps_the_greatest_end_of_each_group"),
        word: Word::Succeeds,
        input: || GROUP_MAX,
        expect: Expect::Value(Cow::Borrowed(
            "GroupBound { from: Step(StepId(0)), keys: [ColumnIndex(1), ColumnIndex(2)], order: ColumnIndex(3), end: Greatest }",
        )),
    },
    Case {
        name: CaseName("group_min_keeps_the_least_end_of_each_group"),
        word: Word::Succeeds,
        input: || GROUP_MIN,
        expect: Expect::Value(Cow::Borrowed(
            "GroupBound { from: Step(StepId(0)), keys: [ColumnIndex(1), ColumnIndex(2)], order: ColumnIndex(3), end: Least }",
        )),
    },
    Case {
        name: CaseName("a_group_bound_with_no_key_is_one_group_after_the_files"),
        word: Word::Succeeds,
        input: || ONE_GROUP,
        expect: Expect::Value(Cow::Borrowed(
            "GroupBound { from: Step(StepId(0)), keys: [], order: ColumnIndex(3), end: Greatest }*",
        )),
    },
    Case {
        name: CaseName("an_order_column_that_is_not_a_number_is_refused"),
        word: Word::Refuses,
        input: || TEXT_ORDER,
        expect: Expect::Error(Cow::Borrowed(
            "`group-min` orders by a number, and its order column `name` is not one (from `<query>` at byte 69)",
        )),
    },
];

fn subject(text: Input) -> Result<Value, Failure> {
    compiled_steps(text, &[OperationKind::GroupBound])
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
