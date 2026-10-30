//! The `closure` item's cases.

use ryo_language::domain::relations::types::{CellKind, Relation, Value as Cell};
use ryo_values::domain::plans::types::QueryOperator;

use ryo_language::domain::symbols::types::Symbols;

use super::super::interface::OperatorsItem as _;
use super::item::ClosureItem;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::table_model::service::push_row;
use crate::domain::table_model::types::{ParameterBinding, Rows, Table};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::relations::types::Integer;
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
        bindings,
        symbols: &mut symbols,
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    ClosureItem
        .compute(operation, &rows, &mut context, &mut out)
        .map(|()| out.count.0)
}

fn reaches_across_a_chain() -> Result<Value, OperatingError> {
    let operation = Operation::Closure {
        edges: FROM,
        source: column(0),
        destination: column(1),
    };
    compute(&operation, &[&numbers(&[&[1, 2], &[2, 3]])], &[])
}

fn closes_over_a_missing_column() -> Result<Value, OperatingError> {
    let operation = Operation::Closure {
        edges: FROM,
        source: column(0),
        destination: column(5),
    };
    compute(&operation, &[&numbers(&[&[1, 2]])], &[])
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_reachable_pair_is_listed"),
        word: Word::Succeeds,
        input: || reaches_across_a_chain,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("a_missing_column_is_refused"),
        word: Word::Refuses,
        input: || closes_over_a_missing_column,
        expect: Expect::Refusal(|error| selects(matches!(error, OperatingError::Table(..)))),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    super::super::fixtures::contract::check(QueryOperator::Closure);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
