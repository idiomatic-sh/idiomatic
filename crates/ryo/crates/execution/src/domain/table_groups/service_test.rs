//! The `table_groups` service's cases, on values alone.

use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::{CellKind, Value as Cell};

use super::service::count_groups;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_groups::types::CountRoom;
use crate::outbound::operators::interface::OperatorsPort;
use ryo_operators::inbound::use_cases::operators::interface::{
    OperatorAnswer, OperatorCall, OperatorsUseCase, PortableOperatorFault,
};
use ryo_values::domain::rows::errors::TableModelError;

#[derive(Debug)]
pub(super) struct TestOperators;
pub(super) static OPERATORS: TestOperators = TestOperators;
impl OperatorsPort for TestOperators {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new()
            .operators(call)
    }
}

use ryo_values::domain::rows::service::push_row;

use crate::domain::{positions::service_test::prepared_order, table_model::types::Rows};
use ryo_derived::domain::contract::types::On;
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// A scenario: what a row asks of the part.
type Input = fn() -> Result<Value, OperatingError>;
/// What a scenario answers: a yes as one, or a count.
type Value = usize;
/// Why a scenario is refused.
type Failure = OperatingError;

/// A table of two numbers a row.
fn pairs(rows: &[[i64; 2]]) -> Table {
    let mut table = Table {
        kinds: vec![CellKind::Number; 2],
        ..Table::default()
    };
    for [left, right] in rows {
        push_row(
            &mut table,
            &[Cell::Number(Integer(*left)), Cell::Number(Integer(*right))],
        );
    }
    table
}

/// Groups come in the order their first row came, each counted by its rows or by its distinct values.
fn counts_the_groups_in_the_order_they_appeared() -> Result<Value, OperatingError> {
    let table = pairs(&[[2, 7], [1, 7], [2, 7], [2, 8]]);
    let on = [ColumnIndex(0)];
    let order = prepared_order(&Rows::Computed(&table), &on)?;
    let (mut groups, mut values) = (Vec::new(), Vec::new());
    count_groups(
        &table,
        On::Columns(&on),
        None,
        &mut CountRoom {
            order: &order,
            groups: &mut groups,
            values: &mut values,
        },
        &OPERATORS,
    )?;
    let by_rows: Vec<(usize, usize)> = groups
        .iter()
        .map(|group| (group.first.0, group.counted.0))
        .collect();
    count_groups(
        &table,
        On::Columns(&on),
        Some(ColumnIndex(1)),
        &mut CountRoom {
            order: &order,
            groups: &mut groups,
            values: &mut values,
        },
        &OPERATORS,
    )?;
    let by_values: Vec<(usize, usize)> = groups
        .iter()
        .map(|group| (group.first.0, group.counted.0))
        .collect();
    Ok(usize::from(
        by_rows == [(0, 3), (1, 1)] && by_values == [(0, 2), (1, 1)],
    ))
}

/// Counting by the distinct values of a column the table lacks is refused.
fn counts_by_a_column_the_table_lacks() -> Result<Value, OperatingError> {
    let table = pairs(&[[2, 7]]);
    let on = [ColumnIndex(0)];
    let order = prepared_order(&Rows::Computed(&table), &on)?;
    let (mut groups, mut values) = (Vec::new(), Vec::new());
    count_groups(
        &table,
        On::Columns(&on),
        Some(ColumnIndex(9)),
        &mut CountRoom {
            order: &order,
            groups: &mut groups,
            values: &mut values,
        },
        &OPERATORS,
    )
    .map(|()| groups.len())
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("groups_are_counted_in_the_order_they_appeared"),
        word: Word::Succeeds,
        input: || counts_the_groups_in_the_order_they_appeared,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_distinct_column_the_table_lacks_is_refused"),
        word: Word::Refuses,
        input: || counts_by_a_column_the_table_lacks,
        expect: Expect::Error(OperatingError::Table(TableModelError::NoSuchColumn {
            index: 9,
        })),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

use ryo_values::domain::rows::types::Table;
