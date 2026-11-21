//! The cases of the `Group(Places)` fill, each run through `made` on a pack the contract's encoding wrote: each distinct
//! row of the input, then its place among its key's rows by the order column, from 0; rows of one order share a place
//! and the next place counts them all. A case says the table's rows (sorted: their order is the index's; `F` for the
//! file's cell, an order as the signed number it is), the fill's counts, and how many cells the table keeps a row.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::filled::errors::FilledError;
use crate::domain::lowered::types::{FillInput, GroupFill, MadeFill, MadeTable};
use crate::domain::made_tables::fixtures::packs::{
    FILE, FIRST, SECOND, of, pipeline_of, read, scan, table,
};
use crate::domain::made_tables::fixtures::{Telling, framed, shown, world_with};
use crate::domain::made_tables::operands::service::file_cell;
use crate::domain::made_tables::sources::service::{made, table_of};
use crate::domain::steps::types::{ColumnList, MadeColumn, MadeTableId, StepOperand};
use std::borrow::Cow;

/// The input a case places.
#[derive(Clone, Copy, Debug)]
enum Placed {
    /// The contract's stated rows: two rows of one key.
    Stated,
    /// One group (no key): two rows of one order, one of them written twice, and a row after them.
    Tied,
    /// Two keys, one of them holding an order below zero and a row written twice.
    Keys,
    /// The same rows keyed by the file's own column: one group.
    ByTheFile,
    /// No row.
    Empty,
}

type Input = Placed;
type Value = Cow<'static, str>;
type Failure = FilledError;

/// An order below zero, as the cell it is.
const BELOW: u64 = (-5_i64).cast_unsigned();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_rows_place_is_how_many_rows_of_its_key_have_a_smaller_order"),
        word: Word::Succeeds,
        input: || Placed::Stated,
        expect: Expect::Value(Cow::Borrowed(
            "1 3 0; 1 5 1 | read 10 given 8 made 2 | the table keeps 3 cells a row",
        )),
    },
    Case {
        name: CaseName("rows_of_one_order_share_a_place_and_a_repeated_row_is_one_row"),
        word: Word::Succeeds,
        input: || Placed::Tied,
        expect: Expect::Value(Cow::Borrowed(
            "1 3 0; 2 3 0; 5 8 2 | read 20 given 15 made 3 | the table keeps 3 cells a row",
        )),
    },
    Case {
        name: CaseName("each_key_is_placed_alone_and_an_order_below_zero_comes_first"),
        word: Word::Succeeds,
        input: || Placed::Keys,
        expect: Expect::Value(Cow::Borrowed(
            "1 -5 0; 1 10 1; 2 20 1; 2 9 0 | read 25 given 19 made 4 | the table keeps 3 cells a row",
        )),
    },
    Case {
        name: CaseName("a_key_that_is_the_file_is_one_group_and_keeps_no_cell"),
        word: Word::Succeeds,
        input: || Placed::ByTheFile,
        expect: Expect::Value(Cow::Borrowed(
            "F 1 -5 0; F 1 10 2; F 2 20 3; F 2 9 1 | read 25 given 19 made 4 | the table keeps 3 cells a row",
        )),
    },
    Case {
        name: CaseName("no_row_in_gives_no_row"),
        word: Word::Succeeds,
        input: || Placed::Empty,
        expect: Expect::Value(Cow::Borrowed(
            " | read 0 given 0 made 0 | the table keeps 3 cells a row",
        )),
    },
];

/// The relation's rows of a case.
fn told(placed: Placed) -> Vec<[u64; 2]> {
    match placed {
        Placed::Stated => vec![[1, 3], [1, 5]],
        Placed::Tied => vec![[1, 3], [2, 3], [1, 3], [5, 8]],
        Placed::Keys | Placed::ByTheFile => vec![[1, 10], [2, 20], [1, BELOW], [2, 9], [1, 10]],
        Placed::Empty => Vec::new(),
    }
}

/// The table of a case: the relation's two columns (after the file, when the file is the key), then the place.
fn tables(placed: Placed) -> Vec<MadeTable> {
    let stages = vec![scan(0), read(FIRST, 0, 1), read(SECOND, 0, 2)];
    let (row, keys, order) = match placed {
        Placed::Stated | Placed::Keys | Placed::Empty => {
            (vec![of(1), of(2)], vec![MadeColumn(0)], 1)
        }
        Placed::Tied => (vec![of(1), of(2)], Vec::new(), 1),
        Placed::ByTheFile => (
            vec![StepOperand::File, of(1), of(2)],
            vec![MadeColumn(0)],
            2,
        ),
    };
    let width = row.len().saturating_add(1);
    vec![table(
        1,
        width,
        MadeFill::Group(
            GroupFill::Places,
            FillInput {
                pipelines: Box::new([pipeline_of(stages, row)]),
                keys: ColumnList(keys.into_boxed_slice()),
                order: Some(MadeColumn(order)),
            },
        ),
    )]
}

fn subject(placed: Input) -> Result<Value, Failure> {
    let mut world = world_with(tables(placed), Vec::new(), &told(placed), Telling::Keyed)?;
    let (mut frame, looped) = framed(&mut world);
    let last = MadeTableId(0);
    let _now = made(&mut frame, looped, last)?;
    let file = file_cell(FILE).0;
    let seen = shown(frame.store, last);
    let mut rows: Vec<String> = seen
        .split("; ")
        .filter(|row| !row.is_empty())
        .map(|row| {
            row.split(' ')
                .map(|cell| {
                    cell.parse::<u64>().map_or_else(
                        |_| cell.to_owned(),
                        |number| {
                            if number == file {
                                "F".to_owned()
                            } else {
                                number.cast_signed().to_string()
                            }
                        },
                    )
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    rows.sort();
    let slot = table_of(frame.store, last);
    let work = slot.map(|slot| slot.work).unwrap_or_default();
    let kept = slot.map_or(0, |slot| slot.rows.width.0);
    Ok(Cow::Owned(format!(
        "{} | read {} given {} made {} | the table keeps {kept} cells a row",
        rows.join("; "),
        work.read.0,
        work.given.0,
        work.made.0
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
