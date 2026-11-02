//! The `project` item's cases.

use ryo_language::domain::relations::types::{CellKind, Relation, Value as Cell};
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plans::types::QueryOperator;

use ryo_language::domain::symbols::types::Symbols;

use super::super::interface::OperatorsItem as _;
use super::item::ProjectItem;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::table_model::service::push_row;
use crate::domain::table_model::types::{ParameterBinding, Rows, Table};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::relations::types::Integer;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plans::types::Operation;

type Value = usize;
type Failure = OperatingError;
/// A scenario: what a row computes with the item, answering how many rows came out.
type Input = fn() -> Result<Value, OperatingError>;

const FROM: PlanInput = PlanInput::Relation(Relation::Function);

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
        prepared_kinds: None,
        prepared_matchers: None,
        stored_indexes: None,
        bindings,
        symbols: (&mut symbols).into(),
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    ProjectItem
        .compute(operation, &rows, &mut context, &mut out)
        .map(|()| out.count.0)
}

fn keeps_one_column_and_drops_repeats() -> Result<Value, OperatingError> {
    let operation = Operation::Project {
        from: FROM,
        columns: vec![column(0)],
        distinct: RepeatedRows::Dropped,
    };
    compute(
        &operation,
        &[&numbers(&[&[1, 10], &[1, 20], &[2, 30]])],
        &[],
    )
}

fn keeps_repeats_when_asked_to() -> Result<Value, OperatingError> {
    let operation = Operation::Project {
        from: FROM,
        columns: vec![column(0)],
        distinct: RepeatedRows::Kept,
    };
    compute(
        &operation,
        &[&numbers(&[&[1, 10], &[1, 20], &[2, 30]])],
        &[],
    )
}

fn projects_a_column_the_input_lacks() -> Result<Value, OperatingError> {
    let operation = Operation::Project {
        from: FROM,
        columns: vec![column(5)],
        distinct: RepeatedRows::Dropped,
    };
    compute(&operation, &[&numbers(&[&[1]])], &[])
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("keeps_named_columns_and_dedups"),
        word: Word::Succeeds,
        input: || keeps_one_column_and_drops_repeats,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("keeps_repeats_when_asked_to"),
        word: Word::Succeeds,
        input: || keeps_repeats_when_asked_to,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("a_missing_column_is_refused"),
        word: Word::Refuses,
        input: || projects_a_column_the_input_lacks,
        expect: Expect::Refusal(|error| selects(matches!(error, OperatingError::Table(..)))),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    super::super::fixtures::contract::check(QueryOperator::Project);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
