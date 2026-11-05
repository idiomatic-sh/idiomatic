//! The `antijoin` item's cases.

use ryo_relations::domain::relations::types::{CellKind, Relation, Value as Cell};
use ryo_values::domain::plans::types::QueryOperator;

use ryo_language::domain::symbols::types::Symbols;

use crate::application::file_dag::testing::TestOperator as _;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::collections::operators::antijoin::item::AntijoinItem;
use crate::domain::table_model::service::push_row;
use crate::domain::table_model::types::Rows;
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as Source;
use ryo_values::domain::plans::types::Operation;

type Value = usize;
type Failure = OperatingError;
/// A scenario: what a row computes with the item, answering how many rows came out.
type Input = fn() -> Result<Value, OperatingError>;

const FROM: Source = Source::Relation(Relation::Function);

fn column(index: u32) -> ColumnIndex {
    ColumnIndex(index)
}

/// A table of numbers, one row for each slice.
fn numbers(rows: &[&[i64]]) -> Table {
    let width = rows.first().map_or(1, |row| row.len());
    let mut table = Table {
        kinds: vec![CellKind::Number; width],
        ..Table::default()
    };
    for row in rows {
        push_row(
            &mut table,
            &row.iter()
                .map(|number| Cell::Number(Integer(*number)))
                .collect::<Vec<_>>(),
        );
    }
    table
}

fn compute(
    operation: &Operation,
    inputs: &[&Table],
    bindings: &[ParameterBinding],
) -> Result<Value, OperatingError> {
    let mut symbols = Symbols::default();
    let rows: Vec<Rows<'_>> = inputs.iter().map(|table| Rows::Computed(table)).collect();
    let mut out = Table::default();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings,
        symbols: (&mut symbols).into(),
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    AntijoinItem
        .compute(operation, &rows, &mut context, &mut out)
        .map(|()| out.count.0)
}

fn keeps_unmatched_left_rows() -> Result<Value, OperatingError> {
    let operation = Operation::Antijoin {
        left: FROM,
        right: FROM,
        left_on: vec![column(0)],
        right_on: vec![column(0)],
    };
    compute(
        &operation,
        &[&numbers(&[&[1, 10], &[2, 20]]), &numbers(&[&[1]])],
        &[],
    )
}

fn antijoins_with_one_input() -> Result<Value, OperatingError> {
    let operation = Operation::Antijoin {
        left: FROM,
        right: FROM,
        left_on: vec![column(0)],
        right_on: vec![column(0)],
    };
    compute(&operation, &[&numbers(&[&[1]])], &[])
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("left_rows_without_a_match_are_kept"),
        word: Word::Succeeds,
        input: || keeps_unmatched_left_rows,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_missing_input_is_refused"),
        word: Word::Refuses,
        input: || antijoins_with_one_input,
        expect: Expect::Refusal(|error| {
            selects(matches!(error, OperatingError::MissingInput { .. }))
        }),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    crate::domain::operator::collections::operators::fixtures::contract::check(
        QueryOperator::Antijoin,
    );
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

use ryo_values::domain::rows::types::Table;

use ryo_values::domain::query_bindings::types::ParameterBinding;
