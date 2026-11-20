//! Lists written in a query, from the compile inbound API: the rows of a `rows` call and the list after `in`.

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

/// Two rows written in the query, under two columns.
const ROWS: &str = r#"let codes = rows(from=[["TRIO110", "ASYNC110"], ["TRIO115", "ASYNC115"]], columns=["code", "canonical"]); emit codes subject code key(canonical);"#;
/// A second row one cell short.
const SHORT_ROW: &str = r#"let codes = rows(from=[["TRIO110", "ASYNC110"], ["TRIO115"]], columns=["code", "canonical"]); emit codes subject code key(canonical);"#;
/// A cell that is not text.
const NUMBER_CELL: &str = r#"let codes = rows(from=[["TRIO110", 110]], columns=["code", "canonical"]); emit codes subject code key(canonical);"#;
/// Membership in a written list of two texts.
const IN_LIST: &str = r#"let a = select name, file from "function" where name in ["main", "run"]; emit a subject file key(name);"#;
/// Membership in a written list of one text.
const IN_ONE: &str = r#"let a = select name, file from "function" where name in ["main"]; emit a subject file key(name);"#;
/// Absence from a written list of two texts.
const NOT_IN_LIST: &str = r#"let a = select name, file from "function" where name not in ["main", "run"]; emit a subject file key(name);"#;
/// Membership in a written list of nothing.
const IN_EMPTY: &str =
    r#"let a = select name, file from "function" where name in []; emit a subject file key(name);"#;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("rows_answers_the_rows_written_once_after_the_files"),
        word: Word::Succeeds,
        input: || ROWS,
        expect: Expect::Value(Cow::Borrowed(
            r#"Rows { source: Written(WrittenRows([WrittenRow([LiteralText("TRIO110"), LiteralText("ASYNC110")]), WrittenRow([LiteralText("TRIO115"), LiteralText("ASYNC115")])])), keys: [TableKey("code"), TableKey("canonical")] }*"#,
        )),
    },
    Case {
        name: CaseName("a_written_row_of_the_wrong_width_is_refused"),
        word: Word::Refuses,
        input: || SHORT_ROW,
        expect: Expect::Error(Cow::Borrowed(
            "row 2 of `rows` has 1 cells, and `columns` names 2 (from `<query>` at byte 0)",
        )),
    },
    Case {
        name: CaseName("a_written_cell_that_is_not_text_is_refused"),
        word: Word::Refuses,
        input: || NUMBER_CELL,
        expect: Expect::Error(Cow::Borrowed(
            "`rows` cannot use its argument `from`: expected a parameter that holds a list of tables, or a list of rows, each a list of text (from `<query>` at byte 0)",
        )),
    },
    Case {
        name: CaseName("a_written_list_after_in_is_each_of_its_values_not_the_first"),
        word: Word::Succeeds,
        input: || IN_LIST,
        expect: Expect::Value(Cow::Borrowed(
            r#"Select { from: Relation(Function), comparisons: [PlanComparison { test: Equal, left: Column(ColumnIndex(1)), right: Literal(Text(LiteralText("main"))), origin: Recorded(ComparisonOriginId(0)) }], kept: [] }; Select { from: Relation(Function), comparisons: [PlanComparison { test: Equal, left: Column(ColumnIndex(1)), right: Literal(Text(LiteralText("run"))), origin: Recorded(ComparisonOriginId(1)) }], kept: [] }"#,
        )),
    },
    Case {
        name: CaseName("a_written_list_of_one_value_is_one_comparison"),
        word: Word::Succeeds,
        input: || IN_ONE,
        expect: Expect::Value(Cow::Borrowed(
            r#"Select { from: Relation(Function), comparisons: [PlanComparison { test: Equal, left: Column(ColumnIndex(1)), right: Literal(Text(LiteralText("main"))), origin: Recorded(ComparisonOriginId(0)) }], kept: [] }"#,
        )),
    },
    Case {
        name: CaseName("a_written_list_after_not_in_excludes_each_of_its_values"),
        word: Word::Succeeds,
        input: || NOT_IN_LIST,
        expect: Expect::Value(Cow::Borrowed(
            r#"Select { from: Relation(Function), comparisons: [PlanComparison { test: NotEqual, left: Column(ColumnIndex(1)), right: Literal(Text(LiteralText("main"))), origin: Recorded(ComparisonOriginId(0)) }, PlanComparison { test: NotEqual, left: Column(ColumnIndex(1)), right: Literal(Text(LiteralText("run"))), origin: Recorded(ComparisonOriginId(1)) }], kept: [] }"#,
        )),
    },
    Case {
        name: CaseName("a_written_list_of_nothing_is_refused"),
        word: Word::Refuses,
        input: || IN_EMPTY,
        expect: Expect::Error(Cow::Borrowed(
            "expected an `import`, `param`, `let`, `observe` or `emit` line at byte 57 (from `<query>` at byte 57)",
        )),
    },
];

fn subject(text: Input) -> Result<Value, Failure> {
    compiled_steps(text, &[OperationKind::Rows, OperationKind::Select])
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
