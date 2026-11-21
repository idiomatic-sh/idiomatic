//! The rows of an ordered match, stated for a small input: one key, the orders 3, 1, 2 on each side, as the operators
//! crate's case `ordered_match_pairs_first_in_first_out` has them, whose pairs are (1, 2) and (2, 3). A left row is its key,
//! its order and a name; a right row is its key and its order. Then which fills the contract states.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowered::service::{match_column, match_left_width};
use crate::domain::lowered::types::FillInput;
use crate::domain::steps::stages::types::Pipeline;
use crate::domain::steps::types::{ColumnList, MadeColumn, Register, StepOperand, StepOperands};
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::table_words::types::ColumnCount;
use std::borrow::Cow;

/// What a row asks.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// The table's rows for the small input, and the operation's output read from them by place.
    Rows,
    /// The left width of a match as the contract states it, and of each way a fill is not one.
    Stated,
}

type Input = Scenario;
type Value = Cow<'static, str>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_ordered_matchs_row_is_the_left_row_then_the_right_row"),
        word: Word::Succeeds,
        input: || Scenario::Rows,
        expect: Expect::Value(Cow::Borrowed(
            "rows [[7, 1, 10, 7, 2], [7, 2, 20, 7, 3]]; output read by place [1, 2, 2, 3]",
        )),
    },
    Case {
        name: CaseName("a_match_is_stated_by_two_ordered_inputs_and_a_table_as_wide_as_both_rows"),
        word: Word::Succeeds,
        input: || Scenario::Stated,
        expect: Expect::Value(Cow::Borrowed(
            "stated Some(3); a table of the output's two columns None; no order None; keys of unequal length None; an order past its row None; pipelines of two widths None; no pipeline None",
        )),
    },
];

/// The key every row of the small input has.
const KEY: u64 = 7;

/// A left row of an order: the key, the order, a name that is ten times the order.
const fn left_row(order: u64) -> [u64; 3] {
    [KEY, order, order * 10]
}

/// A right row of an order: the key, the order.
const fn right_row(order: u64) -> [u64; 2] {
    [KEY, order]
}

/// The table's row of a pair: each cell of each side written at the column `match_column` gives it.
fn row_of(left: [u64; 3], right: [u64; 2]) -> Vec<u64> {
    let left_width = ColumnCount(left.len());
    let mut row = vec![u64::MAX; left.len() + right.len()];
    let sides = [(Side::Left, &left[..]), (Side::Right, &right[..])];
    for (side, cells) in sides {
        for (column, cell) in (0_u16..).zip(cells) {
            let at = match_column(left_width, side, MadeColumn(column)).map(|made| made.0);
            if let Some(place) = at.and_then(|made| row.get_mut(usize::from(made))) {
                *place = *cell;
            }
        }
    }
    row
}

/// The rows of the small input's two pairs, and the operation's output (the left order, the right order) read by place.
fn rows() -> String {
    let left_width = ColumnCount(3);
    let rows = [
        row_of(left_row(1), right_row(2)),
        row_of(left_row(2), right_row(3)),
    ];
    let output = [(Side::Left, MadeColumn(1)), (Side::Right, MadeColumn(1))];
    let read: Vec<u64> = rows
        .iter()
        .flat_map(|row| {
            output.iter().filter_map(|(side, column)| {
                let at = match_column(left_width, *side, *column)?;
                row.get(usize::from(at.0)).copied()
            })
        })
        .collect();
    format!("rows {rows:?}; output read by place {read:?}")
}

/// An input of one pipeline whose rows are `width` wide, keyed on `keys`, ordered by `order`.
fn input(width: u32, keys: &[u16], order: Option<u16>) -> FillInput {
    FillInput {
        pipelines: Box::new([Pipeline {
            stages: Box::new([]),
            row: StepOperands(
                (0..width)
                    .map(|at| StepOperand::Register(Register(at)))
                    .collect(),
            ),
        }]),
        keys: ColumnList(keys.iter().copied().map(MadeColumn).collect()),
        order: order.map(MadeColumn),
    }
}

/// The left width of each fill, the stated one first.
fn stated() -> String {
    let width = |left: &FillInput, right: &FillInput, table| {
        match_left_width(left, right, ColumnCount(table)).map(|width| width.0)
    };
    let (left, right) = (input(3, &[0], Some(1)), input(2, &[0], Some(1)));
    let mut uneven = input(3, &[0], Some(1));
    uneven.pipelines = Box::new([
        Pipeline {
            stages: Box::new([]),
            row: StepOperands(Box::new([StepOperand::File])),
        },
        Pipeline {
            stages: Box::new([]),
            row: StepOperands(Box::new([StepOperand::File, StepOperand::File])),
        },
    ]);
    let mut empty = input(3, &[0], Some(1));
    empty.pipelines = Box::new([]);
    format!(
        "stated {:?}; a table of the output's two columns {:?}; no order {:?}; keys of unequal length {:?}; an order past \
         its row {:?}; pipelines of two widths {:?}; no pipeline {:?}",
        width(&left, &right, 5),
        width(&left, &right, 2),
        width(&left, &input(2, &[0], None), 5),
        width(&left, &input(2, &[0, 1], Some(1)), 5),
        width(&left, &input(2, &[0], Some(2)), 5),
        width(&uneven, &right, 5),
        width(&empty, &right, 5),
    )
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(Cow::Owned(match scenario {
        Scenario::Rows => rows(),
        Scenario::Stated => stated(),
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
