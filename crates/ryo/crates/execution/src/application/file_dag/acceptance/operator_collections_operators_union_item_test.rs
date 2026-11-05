//! The `union` item's cases.

use ryo_relations::domain::relations::types::{CellKind, Relation, Value as Cell};
use ryo_values::domain::plans::types::QueryOperator;

use ryo_language::domain::symbols::types::Symbols;

use crate::application::file_dag::testing::TestOperator as _;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::collections::operators::union::item::UnionItem;
use crate::domain::table_model::types::Rows;
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::Input as Source;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::service::push_row;

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
    compute_table(operation, inputs, bindings).map(|out| out.count.0)
}

fn compute_table(
    operation: &Operation,
    inputs: &[&Table],
    bindings: &[ParameterBinding],
) -> Result<Table, OperatingError> {
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
    UnionItem.compute(operation, &rows, &mut context, &mut out)?;
    Ok(out)
}

fn repeated_alternatives_keep_one_finding_in_both_associations() -> Result<Value, OperatingError> {
    let union = Operation::Union {
        left: FROM,
        right: FROM,
    };
    let a = numbers(&[&[1], &[2]]);
    let b = numbers(&[&[1], &[2]]);
    let c = numbers(&[&[2], &[3]]);
    let d = numbers(&[&[1], &[3]]);
    let ab = compute_table(&union, &[&a, &b], &[])?;
    let cd = compute_table(&union, &[&c, &d], &[])?;
    let balanced = compute_table(&union, &[&ab, &cd], &[])?;
    let abc = compute_table(&union, &[&ab, &c], &[])?;
    let left_deep = compute_table(&union, &[&abc, &d], &[])?;
    assert_eq!(balanced.count, left_deep.count);
    assert_eq!(balanced.cells, left_deep.cells);
    assert_eq!(balanced.cells, numbers(&[&[1], &[2], &[3]]).cells);
    Ok(balanced.count.0)
}

fn two_inputs() -> Result<Value, OperatingError> {
    let operation = Operation::Union {
        left: FROM,
        right: FROM,
    };
    compute(
        &operation,
        &[&numbers(&[&[1], &[2]]), &numbers(&[&[2], &[3]])],
        &[],
    )
}

fn zero_column_inputs() -> Result<Value, OperatingError> {
    let operation = Operation::Union {
        left: FROM,
        right: FROM,
    };
    let mut left = Table::default();
    push_row(&mut left, &[]);
    push_row(&mut left, &[]);
    let mut right = Table::default();
    push_row(&mut right, &[]);
    compute(&operation, &[&left, &right], &[])
}

fn one_input() -> Result<Value, OperatingError> {
    let operation = Operation::Union {
        left: FROM,
        right: FROM,
    };
    compute(&operation, &[&numbers(&[&[1]])], &[])
}

fn unlike_inputs() -> Result<Value, OperatingError> {
    let operation = Operation::Union {
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
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("zero_column_union_keeps_one_distinct_empty_row"),
        word: Word::Succeeds,
        input: || zero_column_inputs,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("repeated_or_alternatives_are_one_finding_in_balanced_or_left_deep_order"),
        word: Word::Succeeds,
        input: || repeated_alternatives_keep_one_finding_in_both_associations,
        expect: Expect::Value(3),
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
        QueryOperator::Union,
    );
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

use ryo_values::domain::rows::types::Table;

use ryo_values::domain::query_bindings::types::ParameterBinding;
