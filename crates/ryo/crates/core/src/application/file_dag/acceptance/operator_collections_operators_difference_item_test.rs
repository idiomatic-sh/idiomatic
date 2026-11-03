//! The `difference` item's cases.

use ryo_language::domain::relations::types::{CellKind, Relation, Value as Cell};
use ryo_values::domain::plans::types::QueryOperator;

use ryo_language::domain::symbols::types::Symbols;

use crate::application::file_dag::testing::TestOperator as _;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::collections::operators::difference::item::DifferenceItem;
use crate::domain::table_model::service::push_row;
use crate::domain::table_model::types::{ParameterBinding, Rows, Table};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::relations::types::Integer;
use ryo_values::domain::plan_parts::types::Input as Source;
use ryo_values::domain::plans::types::Operation;

type Value = usize;
type Failure = OperatingError;
/// A scenario: what a row computes with the item, answering how many rows came out.
type Input = fn() -> Result<Value, OperatingError>;

const FROM: Source = Source::Relation(Relation::Function);

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
        bindings,
        symbols: (&mut symbols).into(),
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    DifferenceItem
        .compute(operation, &rows, &mut context, &mut out)
        .map(|()| out.count.0)
}

fn two_inputs() -> Result<Value, OperatingError> {
    let operation = Operation::Difference {
        left: FROM,
        right: FROM,
    };
    compute(
        &operation,
        &[&numbers(&[&[1], &[2]]), &numbers(&[&[2], &[3]])],
        &[],
    )
}

fn one_input() -> Result<Value, OperatingError> {
    let operation = Operation::Difference {
        left: FROM,
        right: FROM,
    };
    compute(&operation, &[&numbers(&[&[1]])], &[])
}

fn unlike_inputs() -> Result<Value, OperatingError> {
    let operation = Operation::Difference {
        left: FROM,
        right: FROM,
    };
    compute(&operation, &[&numbers(&[&[1]]), &numbers(&[&[1, 2]])], &[])
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("rows_are_combined_as_the_operator_says"),
        word: Word::Succeeds,
        input: || two_inputs,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_missing_input_is_refused"),
        word: Word::Refuses,
        input: || one_input,
        expect: Expect::Refusal(|error| {
            selects(matches!(error, OperatingError::MissingInput { .. }))
        }),
    },
    Case {
        name: CaseName("inputs_with_other_columns_are_refused"),
        word: Word::Refuses,
        input: || unlike_inputs,
        expect: Expect::Error(OperatingError::ShapeMismatch),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    crate::domain::operator::collections::operators::fixtures::contract::check(
        QueryOperator::Difference,
    );
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
