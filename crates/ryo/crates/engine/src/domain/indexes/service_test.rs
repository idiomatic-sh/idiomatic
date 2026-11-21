//! The `indexes` service's cases: every bound column is one key; an index is its site's and its file's, whatever list its
//! columns were handed in; a key's rows are ascending, or sorted by the order column; a range reads its bounds as written.

use super::reading::service::{read, rows_of, run_between, run_of};
use super::service::{begin, finish, key_column, state_of};
use super::types::{ColumnView, FileStamp, IndexArena, IndexBuilt, RangeEnd};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::IndexSite;
use crate::domain::steps::types::BoundKind;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;
use std::borrow::Cow;

/// What a case builds and asks.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// The rows of the key (1, 2) in an index by two columns.
    BothColumns,
    /// Two indexes built one after the other from one list of columns at one address: the rows of key 1 in each.
    SameAddress,
    /// An index built for one file, asked whether it is built for the next.
    NextFile,
    /// The rows of key 1 in an index ordered by a second column.
    Ordered,
    /// The rows of key 1 between two bounds: (low, low is in, high, high is in), a bound of `u64::MAX` being absent.
    Between(u64, bool, u64, bool),
    /// A key no row holds.
    Missing,
    /// One column whose cells lie far apart, and one whose cells lie close: the rows of one key in each, and of a cell
    /// outside the close ones.
    FarAndClose,
}

type Input = Asked;
type Value = Cow<'static, [u32]>;
type Failure = ();

/// The first column of the source: the key.
const FIRST: [u64; 6] = [1, 2, 1, 1, 2, 1];
/// The second column: a second key cell, or the order.
const SECOND: [u64; 6] = [2, 2, 9, 2, 1, 5];

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_bound_column_is_one_key"),
        word: Word::Succeeds,
        input: || Asked::BothColumns,
        expect: Expect::Value(Cow::Borrowed(&[0, 3])),
    },
    Case {
        name: CaseName("an_index_is_named_by_its_site_never_by_the_address_of_its_columns"),
        word: Word::Succeeds,
        input: || Asked::SameAddress,
        expect: Expect::Value(Cow::Borrowed(&[0, 2, 3, 5, 99, 4])),
    },
    Case {
        name: CaseName("an_index_of_the_last_file_is_not_built_for_the_next"),
        word: Word::Succeeds,
        input: || Asked::NextFile,
        expect: Expect::Value(Cow::Borrowed(&[1, 0])),
    },
    Case {
        name: CaseName("an_ordered_index_sorts_a_keys_rows_by_the_order_column"),
        word: Word::Succeeds,
        input: || Asked::Ordered,
        expect: Expect::Value(Cow::Borrowed(&[0, 3, 5, 2])),
    },
    Case {
        name: CaseName("a_range_with_both_ends_in"),
        word: Word::Succeeds,
        input: || Asked::Between(2, true, 5, true),
        expect: Expect::Value(Cow::Borrowed(&[0, 3, 5])),
    },
    Case {
        name: CaseName("a_strict_bound_leaves_its_own_rows_out"),
        word: Word::Succeeds,
        input: || Asked::Between(2, false, 9, false),
        expect: Expect::Value(Cow::Borrowed(&[5])),
    },
    Case {
        name: CaseName("an_absent_bound_leaves_its_side_open"),
        word: Word::Succeeds,
        input: || Asked::Between(u64::MAX, false, 5, false),
        expect: Expect::Value(Cow::Borrowed(&[0, 3])),
    },
    Case {
        name: CaseName("an_absent_high_bound_reaches_the_keys_last_row"),
        word: Word::Succeeds,
        input: || Asked::Between(2, true, u64::MAX, false),
        expect: Expect::Value(Cow::Borrowed(&[0, 3, 5, 2])),
    },
    Case {
        name: CaseName("cells_far_apart_and_cells_close_together_find_the_same_rows"),
        word: Word::Succeeds,
        input: || Asked::FarAndClose,
        expect: Expect::Value(Cow::Borrowed(&[1, 4, 99, 1, 4, 99])),
    },
    Case {
        name: CaseName("a_key_no_row_holds_has_no_rows"),
        word: Word::Succeeds,
        input: || Asked::Missing,
        expect: Expect::Value(Cow::Borrowed(&[])),
    },
];

/// A column of cells.
fn cells(numbers: &[u64]) -> Vec<CellWord> {
    numbers.iter().map(|number| CellWord(*number)).collect()
}

/// A column that lies cell after cell.
fn plain(column: &[CellWord]) -> ColumnView<'_> {
    ColumnView {
        cells: column,
        first: Count(0),
        stride: Count(1),
    }
}

/// Builds the index of `site` by the listed columns.
fn built(arena: &mut IndexArena, site: u32, columns: &[&[CellWord]], order: Option<&[CellWord]>) {
    begin(arena, Count(FIRST.len()), Count(columns.len()));
    for (at, column) in columns.iter().enumerate() {
        key_column(arena, Count(at), plain(column));
    }
    finish(arena, IndexSite(site), FileStamp(1), order.map(plain));
}

fn subject(asked: Input) -> Result<Value, Failure> {
    let (first, second) = (cells(&FIRST), cells(&SECOND));
    let mut arena = IndexArena::default();
    let found = |arena: &IndexArena, site: u32, key: &[u64]| -> Vec<u32> {
        let index = read(arena, IndexSite(site));
        rows_of(index, run_of(index, &cells(key)))
            .iter()
            .map(|row| row.0)
            .collect()
    };
    Ok(Cow::Owned(match asked {
        Asked::BothColumns => {
            built(&mut arena, 0, &[&first, &second], None);
            found(&arena, 0, &[1, 2])
        }
        Asked::SameAddress => {
            // One list, at one address, names the first column and then the second: each site keeps its own index.
            let mut list: Vec<&[CellWord]> = vec![&first];
            built(&mut arena, 0, &list, None);
            if let Some(column) = list.first_mut() {
                *column = &second;
            }
            built(&mut arena, 1, &list, None);
            let mut both = found(&arena, 0, &[1]);
            both.push(99);
            both.extend(found(&arena, 1, &[1]));
            both
        }
        Asked::NextFile => {
            built(&mut arena, 0, &[&first], None);
            vec![
                u32::from(state_of(&arena, IndexSite(0), FileStamp(1)) == IndexBuilt::Built),
                u32::from(state_of(&arena, IndexSite(0), FileStamp(2)) == IndexBuilt::Built),
            ]
        }
        Asked::Ordered => {
            built(&mut arena, 0, &[&first], Some(&second));
            found(&arena, 0, &[1])
        }
        Asked::Between(low, low_in, high, high_in) => {
            built(&mut arena, 0, &[&first], Some(&second));
            let index = read(&arena, IndexSite(0));
            let bound = |cell: u64, is_in: bool| {
                (cell != u64::MAX).then_some(RangeEnd {
                    cell: CellWord(cell),
                    end: if is_in {
                        BoundKind::Inclusive
                    } else {
                        BoundKind::Strict
                    },
                })
            };
            let run = run_between(
                index,
                run_of(index, &cells(&[1])),
                plain(&second),
                bound(low, low_in),
                bound(high, high_in),
            );
            rows_of(index, run).iter().map(|row| row.0).collect()
        }
        Asked::FarAndClose => {
            let far = cells(&[7, 9_000_000_000, 7, 8, 9_000_000_000, 7]);
            built(&mut arena, 0, &[&far], None);
            built(&mut arena, 1, &[&first], None);
            let mut both = found(&arena, 0, &[9_000_000_000]);
            both.push(99);
            both.extend(found(&arena, 1, &[2]));
            both.push(99);
            both.extend(found(&arena, 1, &[9_000_000_000]));
            both
        }
        Asked::Missing => {
            built(&mut arena, 0, &[&first, &second], None);
            found(&arena, 0, &[2, 9])
        }
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
