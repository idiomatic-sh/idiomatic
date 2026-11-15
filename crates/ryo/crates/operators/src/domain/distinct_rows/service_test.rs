//! A repeated row is kept once, the first in place, and the rows keep their order.
use super::service::kept_once;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::rows::types::Table;
use std::borrow::Cow;

type Input = &'static [i64];
/// The numbers of the rows kept, in order.
type Value = Cow<'static, [i64]>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_repeated_row_is_kept_once_the_first_in_place"),
        word: Word::Succeeds,
        input: || &[3, 1, 3, 2, 1],
        expect: Expect::Value(Cow::Borrowed(&[3, 1, 2])),
    },
    Case {
        name: CaseName("rows_without_repeats_are_kept_as_they_are"),
        word: Word::Succeeds,
        input: || &[1, 2, 3],
        expect: Expect::Value(Cow::Borrowed(&[1, 2, 3])),
    },
];

fn subject(numbers: Input) -> Result<Value, Failure> {
    let mut table = Table {
        kinds: vec![CellKind::Number],
        ..Table::default()
    };
    for number in numbers {
        push_row(&mut table, &[Cell::Number(Integer(*number))]);
    }
    kept_once(&mut table);
    Ok(Cow::Owned(
        table
            .cells
            .iter()
            .filter_map(|cell| {
                if let Cell::Number(Integer(number)) = cell {
                    Some(*number)
                } else {
                    None
                }
            })
            .collect(),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
