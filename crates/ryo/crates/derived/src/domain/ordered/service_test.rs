//! The `ordered` service's cases, on values alone.

use super::service::build;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::fault::types::DerivedFault;
use crate::domain::row_cells::types::ValueTexts;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::row_texts::types::CellText;
use ryo_values::domain::rows::types::{RowCount, Table};
use std::borrow::Cow;

#[derive(Clone, Copy)]
enum Scenario {
    SidesSortByOrder,
    DuplicateOrderIsRefused,
    OrderColumnMustBeANumber,
}
type Input = Scenario;
type Value = Cow<'static, [usize]>;
type Failure = DerivedFault;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_side_sorts_by_key_then_order"),
        word: Word::Succeeds,
        input: || Scenario::SidesSortByOrder,
        expect: Expect::Value(Cow::Borrowed(&[1, 0])),
    },
    Case {
        name: CaseName("two_rows_with_one_key_and_one_order_are_refused"),
        word: Word::Refuses,
        input: || Scenario::DuplicateOrderIsRefused,
        expect: Expect::Error(DerivedFault::DuplicateOrder),
    },
    Case {
        name: CaseName("an_order_column_that_is_not_a_number_is_refused"),
        word: Word::Refuses,
        input: || Scenario::OrderColumnMustBeANumber,
        expect: Expect::Error(DerivedFault::ShapeMismatch),
    },
];

struct NoTexts;
impl ValueTexts for NoTexts {
    fn text(&self, _value: Cell) -> Option<CellText<'_>> {
        None
    }
}

fn rows(kinds: [CellKind; 2], cells: &[i64]) -> Table {
    Table {
        kinds: kinds.to_vec(),
        cells: cells
            .iter()
            .map(|cell| Cell::Number(Integer(*cell)))
            .collect(),
        count: RowCount(cells.len() / 2),
    }
}

/// Calls the part on one input.
fn subject(input: Input) -> Result<Value, Failure> {
    let numbers = [CellKind::Number, CellKind::Number];
    let (left, order_kind) = match input {
        Scenario::SidesSortByOrder => (rows(numbers, &[1, 9, 1, 3]), numbers),
        Scenario::DuplicateOrderIsRefused => (rows(numbers, &[1, 3, 1, 3]), numbers),
        Scenario::OrderColumnMustBeANumber => {
            (rows([CellKind::Number, CellKind::Flag], &[1, 3]), numbers)
        }
    };
    let right = rows(order_kind, &[1, 1]);
    let matching = build(
        &left,
        &right,
        &[ColumnIndex(0)],
        &[ColumnIndex(0)],
        ColumnIndex(1),
        ColumnIndex(1),
        &NoTexts,
        &mut Vec::new(),
    )?;
    Ok(Cow::Owned(
        matching.left.iter().map(|row| row.row.0).collect(),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
