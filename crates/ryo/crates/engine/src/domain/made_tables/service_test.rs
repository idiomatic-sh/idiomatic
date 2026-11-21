//! The `made_tables` service's cases: a write appends every row or only the rows the table does not hold; a counted grouping
//! gives one row a key; a sorted table keeps a key's rows together and ascending.

use super::service::{clear_rows, count_groups, set_keys, sort_groups, write, write_new};
use super::types::{GroupRoom, TableRows};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::stages::types::BatchValue;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::borrow::Cow;

/// The fill a case runs.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// Two batches written, every row kept.
    Write,
    /// The same two batches written with repeats dropped.
    WriteNew,
    /// The rows counted by their first column.
    Count,
    /// The different cells of the second column counted by the first.
    CountDistinct,
    /// The rows counted by a column the input does not have.
    CountNoColumn,
    /// The rows sorted within their first column by their second, and the orders kept beside them.
    Sorted,
}

type Input = Asked;
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The first column written.
const FIRST: [u64; 4] = [3, 1, 3, 1];
/// The second column written.
const SECOND: [u64; 4] = [9, 4, 9, 2];

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_write_appends_every_row"),
        word: Word::Succeeds,
        input: || Asked::Write,
        expect: Expect::Value(Cow::Borrowed(&[
            8, 3, 9, 7, 1, 4, 7, 3, 9, 7, 1, 2, 7, 3, 9, 7, 1, 4, 7, 3, 9, 7, 1, 2, 7,
        ])),
    },
    Case {
        name: CaseName("a_write_with_repeats_dropped_keeps_each_row_once"),
        word: Word::Succeeds,
        input: || Asked::WriteNew,
        expect: Expect::Value(Cow::Borrowed(&[3, 3, 9, 7, 1, 4, 7, 1, 2, 7])),
    },
    Case {
        name: CaseName("a_counted_grouping_gives_each_key_and_its_rows"),
        word: Word::Succeeds,
        input: || Asked::Count,
        expect: Expect::Value(Cow::Borrowed(&[2, 1, 2, 3, 2])),
    },
    Case {
        name: CaseName("a_distinct_count_counts_different_cells"),
        word: Word::Succeeds,
        input: || Asked::CountDistinct,
        expect: Expect::Value(Cow::Borrowed(&[2, 1, 2, 3, 1])),
    },
    Case {
        name: CaseName("a_sorted_table_keeps_a_keys_rows_together_and_ascending"),
        word: Word::Succeeds,
        input: || Asked::Sorted,
        expect: Expect::Value(Cow::Borrowed(&[
            4, 1, 2, 7, 1, 4, 7, 3, 9, 7, 3, 9, 7, 100, 2, 4, 9, 9,
        ])),
    },
    Case {
        name: CaseName("a_key_that_names_no_column_is_a_fault"),
        word: Word::Refuses,
        input: || Asked::CountNoColumn,
        expect: Expect::Error(()),
    },
];

/// A column of cells.
fn cells(numbers: &[u64]) -> Vec<CellWord> {
    numbers.iter().map(|number| CellWord(*number)).collect()
}

/// A table's count, then its cells; the rows of a grouping fill sorted, since their order is the index's.
fn shown(table: &TableRows, sort: bool) -> Vec<u64> {
    let mut rows: Vec<Vec<u64>> = table
        .cells
        .chunks(table.width.0.max(1))
        .map(|row| row.iter().map(|cell| cell.0).collect())
        .collect();
    if sort {
        rows.sort();
    }
    let mut all = vec![u64::try_from(table.count.0).unwrap_or(u64::MAX)];
    all.extend(rows.into_iter().flatten());
    all
}

fn subject(asked: Input) -> Result<Value, Failure> {
    let (first, second) = (cells(&FIRST), cells(&SECOND));
    let sources = [
        BatchValue::Column(&first),
        BatchValue::Column(&second),
        BatchValue::Constant(CellWord(7)),
    ];
    let mut input = TableRows::default();
    clear_rows(&mut input, Count(3));
    write(&mut input, &sources, Count(first.len()));
    let mut room = GroupRoom::default();
    let mut out = TableRows::default();
    Ok(Cow::Owned(match asked {
        Asked::Write => {
            write(&mut input, &sources, Count(first.len()));
            shown(&input, false)
        }
        Asked::WriteNew => {
            clear_rows(&mut out, Count(3));
            write_new(&mut out, &sources, Count(first.len()));
            write_new(&mut out, &sources, Count(first.len()));
            shown(&out, false)
        }
        Asked::Count | Asked::CountDistinct | Asked::CountNoColumn => {
            clear_rows(&mut out, Count(2));
            set_keys(
                &mut room,
                std::iter::once(ColumnIndex(if matches!(asked, Asked::CountNoColumn) {
                    5
                } else {
                    0
                })),
            );
            let distinct = matches!(asked, Asked::CountDistinct).then_some(Count(1));
            count_groups(&input, distinct, &mut room, &mut out).map_err(|_| ())?;
            shown(&out, true)
        }
        Asked::Sorted => {
            clear_rows(&mut out, Count(3));
            set_keys(&mut room, std::iter::once(ColumnIndex(0)));
            sort_groups(&input, Count(1), &mut room, &mut out);
            let mut all = shown(&out, false);
            all.push(100);
            let orders: Vec<u64> = out
                .orders
                .iter()
                .map(|order| order.0.cast_unsigned())
                .collect();
            all.extend(orders);
            all
        }
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

mod built;
mod calls;
mod file_columns;
mod fills;
mod held_columns;
mod matches;
mod node_indexes;
mod pipelines;
mod relations;
mod sizes;
mod sources;
mod verdict_bounds;
mod verdicts;
mod walks;
