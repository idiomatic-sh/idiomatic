//! Behavioral contract shared by the injected operator adapter and test fake, driven through a call the engine makes.
use super::reduce_count::{Groups, table};
use crate::outbound::operators::interface::OperatorsPort;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::domain::reduction::types::GroupColumns;
use ryo_operators::inbound::use_cases::operators::types::{OperatorAnswer, OperatorCall};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::RowIndex;
use std::borrow::Cow;

/// Rows of two numbers, already in the prepared key order the engine hands the reduction, and the key columns they are counted by.
type Input = (Vec<[i64; 2]>, Vec<ColumnIndex>);
/// Each group's first row and how many rows it counted, in first-occurrence order.
type Value = Cow<'static, [(usize, usize)]>;
type Failure = PortableOperatorFault;

/// Observable group counts and key-column refusal across the port.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("rows_in_prepared_key_order_are_counted_into_their_groups"),
        word: Word::Succeeds,
        input: || (vec![[1, 7], [2, 7], [2, 8], [2, 7]], vec![ColumnIndex(0)]),
        expect: Expect::Value(Cow::Borrowed(&[(0, 1), (1, 3)])),
    },
    Case {
        name: CaseName("an_absent_key_column_is_refused"),
        word: Word::Refuses,
        input: || (vec![[2, 7]], vec![ColumnIndex(9)]),
        expect: Expect::Error(PortableOperatorFault::NoSuchColumn {
            index: ColumnIndex(9),
        }),
    },
];

/// Counts `rows`, taken in their own order as the prepared key order, by `keys` through `adapter` into `target`.
///
/// # Errors
///
/// The fault the port answers.
pub fn count(
    adapter: &impl OperatorsPort,
    rows: &[[i64; 2]],
    keys: &[ColumnIndex],
    target: &mut Groups,
) -> Result<OperatorAnswer, PortableOperatorFault> {
    let table = table(rows);
    let order: Vec<RowIndex> = (0..rows.len()).map(RowIndex).collect();
    adapter.operators(OperatorCall::ReduceCount {
        table: &table,
        keys: GroupColumns::Columns(keys),
        distinct: None,
        order: &order,
        values: &mut Vec::new(),
        target,
    })
}

/// Run the same observable cases for every implementation of the engine port.
///
/// # Panics
///
/// When the implementation changes the group counts or their refusal.
pub fn check(adapter: &impl OperatorsPort) {
    let subject = |(rows, keys): Input| {
        let mut groups = Groups::default();
        let _answer = count(adapter, &rows, &keys, &mut groups)?;
        Ok(Cow::Owned(groups.0))
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}
