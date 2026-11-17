//! A row set keeps each first row once, in first-seen order, tells two rows of one hash apart by their cells, and holds each
//! row of the side it was built from and no other.
use super::value::{FirstSeen, RowSet, SetEntry, hash_of};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::physical_plan::types::CellStart;
use crate::domain::physical_plan::types::keyed_rows::value::IndexedRow;
use crate::domain::table_scratch::types::SetMembership;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::service::{push_row, row_in};
use ryo_values::domain::rows::types::{RowCount, Table};
use std::borrow::Cow;

/// What a case keeps in a row set.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// Rows of the numbers, each kept when it is the first of its kind.
    Seen(&'static [i64]),
    /// 1,000 rows of 600 numbers: each row's number is its place modulo 600.
    SixHundredOfAThousand,
    /// A row of 2 after a row of 1 planted under the hash of a row of 2.
    Collided,
    /// A set built from rows of the first numbers, asked for each number of the second.
    Built(&'static [i64], &'static [i64]),
}
type Input = Given;
type Value = Observation;
type Failure = ();

/// What a row set answered.
#[derive(Debug, PartialEq, Eq)]
enum Observation {
    /// The numbers of the rows kept, in order, and how many rows the set keeps.
    Kept {
        numbers: Cow<'static, [i64]>,
        set: RowCount,
    },
    /// How many rows the table and the set keep, and the first and last numbers kept.
    Counted {
        rows: RowCount,
        set: RowCount,
        ends: (Option<i64>, Option<i64>),
    },
    /// The set's first-seen answer for the last row.
    Last(FirstSeen),
    /// Whether the set holds each asked number, in order, and how many rows it keeps.
    Holds(Cow<'static, [SetMembership]>, RowCount),
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_repeated_row_is_not_kept_twice"),
        word: Word::Succeeds,
        input: || Given::Seen(&[4, 4, 4]),
        expect: Expect::Value(Observation::Kept {
            numbers: Cow::Borrowed(&[4]),
            set: RowCount(1),
        }),
    },
    Case {
        name: CaseName("first_rows_keep_their_order"),
        word: Word::Succeeds,
        input: || Given::Seen(&[3, 1, 3, 2, 1]),
        expect: Expect::Value(Observation::Kept {
            numbers: Cow::Borrowed(&[3, 1, 2]),
            set: RowCount(3),
        }),
    },
    Case {
        name: CaseName("a_distinct_keeps_each_first_row_and_no_link"),
        word: Word::Succeeds,
        input: || Given::SixHundredOfAThousand,
        expect: Expect::Value(Observation::Counted {
            rows: RowCount(600),
            set: RowCount(600),
            ends: (Some(0), Some(599)),
        }),
    },
    Case {
        name: CaseName("equal_hash_unequal_rows_are_both_kept"),
        word: Word::Succeeds,
        input: || Given::Collided,
        expect: Expect::Value(Observation::Last(FirstSeen::First)),
    },
    Case {
        name: CaseName("a_built_side_holds_each_of_its_rows"),
        word: Word::Succeeds,
        input: || Given::Built(&[5, 6, 5, 7], &[5, 6, 7, 8]),
        expect: Expect::Value(Observation::Holds(
            Cow::Borrowed(&[
                SetMembership::In,
                SetMembership::In,
                SetMembership::In,
                SetMembership::Out,
            ]),
            RowCount(3),
        )),
    },
];

/// A table of one number column, of no row.
fn numbers_table() -> Table {
    Table {
        kinds: vec![CellKind::Number],
        ..Table::default()
    }
}

/// Appends a row of `number` to `table` and keeps it when the set sees it first, as the distinct kernel does.
fn seen(table: &mut Table, set: &mut RowSet, number: i64) -> FirstSeen {
    let start = CellStart(table.cells.len());
    table.cells.push(Cell::Number(Integer(number)));
    let first = set.first_seen(table, start);
    if first == FirstSeen::First {
        table.count = RowCount(table.count.0 + 1);
    } else {
        table.cells.truncate(start.0);
    }
    first
}

/// The numbers of a table's rows, in order.
fn numbers_of(table: &Table) -> Vec<i64> {
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
        .collect()
}

fn subject(given: Input) -> Result<Value, Failure> {
    let mut set = RowSet::default();
    let mut table = numbers_table();
    match given {
        Given::Seen(numbers) => {
            for number in numbers {
                let _: FirstSeen = seen(&mut table, &mut set, *number);
            }
        }
        Given::SixHundredOfAThousand => {
            for at in 0..1_000 {
                let _: FirstSeen = seen(&mut table, &mut set, at % 600);
            }
            let numbers = numbers_of(&table);
            return Ok(Observation::Counted {
                rows: table.count,
                set: set.count(),
                ends: (numbers.first().copied(), numbers.last().copied()),
            });
        }
        Given::Collided => {
            let _: FirstSeen = seen(&mut table, &mut set, 1);
            let two = [Cell::Number(Integer(2))];
            let hash = hash_of(&two);
            set.kept.clear();
            let _: hashbrown::hash_table::OccupiedEntry<'_, SetEntry> = set.kept.insert_unique(
                hash.0,
                SetEntry {
                    hash,
                    row: IndexedRow(0),
                },
                |kept| kept.hash.0,
            );
            return Ok(Observation::Last(seen(&mut table, &mut set, 2)));
        }
        Given::Built(side, asked) => {
            for number in side {
                push_row(&mut table, &[Cell::Number(Integer(*number))]);
            }
            let cell = |row, column: ColumnIndex| {
                row_in(&table, row)?
                    .get(usize::try_from(column.0).ok()?)
                    .copied()
            };
            set.built(table.count, &table.kinds, cell).map_err(|_| ())?;
            let holds = asked
                .iter()
                .map(|number| set.holds(&table.kinds, cell, &[Cell::Number(Integer(*number))]))
                .collect();
            return Ok(Observation::Holds(Cow::Owned(holds), set.count()));
        }
    }
    Ok(Observation::Kept {
        numbers: Cow::Owned(numbers_of(&table)),
        set: set.count(),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
