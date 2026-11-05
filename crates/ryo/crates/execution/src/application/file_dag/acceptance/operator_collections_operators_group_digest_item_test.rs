//! The `group_digest` item's cases.

use ryo_relations::domain::relations::types::{CellKind, Relation, Value as Cell};
use ryo_values::domain::plans::types::QueryOperator;

use ryo_language::domain::symbols::types::Symbols;

use crate::application::file_dag::testing::TestOperator as _;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::collections::operators::group_digest::item::GroupDigestItem;

use crate::domain::table_model::service::{push_row, row_in};
use crate::domain::table_model::types::{Rows};
use crate::domain::table_scratch::types::OperatorScratch;
use {ryo_values::domain::files::types::SourceView};
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plans::types::Operation;

type Value = usize;
type Failure = OperatingError;
/// A scenario: what a row computes with the item, answering a count.
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

/// The digest of every group of `input` by column 0 over column 1, with how many values it has.
fn digests(input: &Table, bindings: &[ParameterBinding]) -> Result<Table, OperatingError> {
    let mut symbols = Symbols::default();
    let rows = [Rows::Computed(input)];
    let mut out = Table::default();
    let operation = Operation::GroupDigest {
        from: FROM,
        keys: vec![column(0)],
        over: column(1),
    };
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
    GroupDigestItem.compute(&operation, &rows, &mut context, &mut out)?;
    Ok(out)
}

/// The cells of the group at `index` of `table`: key, digest, size.
fn group(table: &Table, index: usize) -> Vec<Cell> {
    row_in(table, RowIndex(index))
        .map(<[Cell]>::to_vec)
        .unwrap_or_default()
}

fn one_row_per_group_with_a_digest_and_a_size() -> Result<Value, OperatingError> {
    let out = digests(&numbers(&[&[1, 10], &[1, 20], &[2, 30]]), &[])?;
    Ok(out.count.0 * 10 + out.kinds.len())
}

fn equal_sets_give_equal_digests_whatever_the_order_or_the_repeats() -> Result<Value, OperatingError>
{
    let out = digests(
        &numbers(&[&[1, 10], &[1, 20], &[2, 20], &[2, 10], &[2, 10], &[2, 20]]),
        &[],
    )?;
    Ok(usize::from(group(&out, 0)[1..] == group(&out, 1)[1..]))
}

fn different_sets_give_different_digests() -> Result<Value, OperatingError> {
    let out = digests(&numbers(&[&[1, 10], &[1, 20], &[2, 10], &[2, 30]]), &[])?;
    Ok(usize::from(
        group(&out, 0)[1] != group(&out, 1)[1] && group(&out, 0)[2] == group(&out, 1)[2],
    ))
}

fn a_set_of_one_value_is_not_a_set_of_two() -> Result<Value, OperatingError> {
    let out = digests(&numbers(&[&[1, 10], &[2, 10], &[2, 20]]), &[])?;
    Ok(usize::from(
        group(&out, 0)[1] != group(&out, 1)[1]
            && group(&out, 0)[2] == Cell::Number(Integer(1))
            && group(&out, 1)[2] == Cell::Number(Integer(2)),
    ))
}

fn digesting_without_an_input() -> Result<Value, OperatingError> {
    let operation = Operation::GroupDigest {
        from: FROM,
        keys: vec![column(0)],
        over: column(1),
    };
    let mut symbols = Symbols::default();
    let mut out = Table::default();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    GroupDigestItem
        .compute(&operation, &[], &mut context, &mut out)
        .map(|()| out.count.0)
}

fn digesting_a_column_the_input_lacks() -> Result<Value, OperatingError> {
    let operation = Operation::GroupDigest {
        from: FROM,
        keys: vec![column(0)],
        over: column(9),
    };
    let input = numbers(&[&[1, 10]]);
    let rows = [Rows::Computed(&input)];
    let mut symbols = Symbols::default();
    let mut out = Table::default();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    GroupDigestItem
        .compute(&operation, &rows, &mut context, &mut out)
        .map(|()| out.count.0)
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("one_row_per_group_with_a_digest_and_a_size"),
        word: Word::Succeeds,
        input: || one_row_per_group_with_a_digest_and_a_size,
        expect: Expect::Value(23),
    },
    Case {
        name: CaseName("equal_sets_give_equal_digests_whatever_the_order_or_the_repeats"),
        word: Word::Succeeds,
        input: || equal_sets_give_equal_digests_whatever_the_order_or_the_repeats,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("different_sets_give_different_digests"),
        word: Word::Succeeds,
        input: || different_sets_give_different_digests,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_set_of_one_value_is_not_a_set_of_two"),
        word: Word::Succeeds,
        input: || a_set_of_one_value_is_not_a_set_of_two,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_missing_input_is_refused"),
        word: Word::Refuses,
        input: || digesting_without_an_input,
        expect: Expect::Refusal(|error| {
            selects(matches!(error, OperatingError::MissingInput { .. }))
        }),
    },
    Case {
        name: CaseName("a_column_the_input_lacks_is_refused"),
        word: Word::Refuses,
        input: || digesting_a_column_the_input_lacks,
        expect: Expect::Error(OperatingError::Table(
            ryo_values::domain::rows::errors::TableModelError::NoSuchColumn { index: 9 },
        )),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    crate::domain::operator::collections::operators::fixtures::contract::check(
        QueryOperator::GroupDigest,
    );
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

use ryo_values::domain::rows::types::{RowIndex, Table};

use ryo_values::domain::query_bindings::types::{ParameterBinding};
