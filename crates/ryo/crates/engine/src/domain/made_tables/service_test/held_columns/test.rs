//! The cases of a column handed to its reader for the rest of the file (`reading::service::column_held`): a reader
//! asks once where a column of a made table or of a relation lies, keeps the answer while the store changes, and reads
//! the cells itself. A case says where each column lies and the cells read through it for the relation's five rows and
//! for one row past them (`F` for the file's cell).

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::{RelationNumber, Short};
use crate::domain::filled::errors::FilledError;
use crate::domain::lowered::types::{MadeFill, MadeTable};
use crate::domain::made_tables::fixtures::packs::{
    FILE, FIRST, SECOND, of, pipeline, read, rows, scan, table,
};
use crate::domain::made_tables::fixtures::{Telling, framed, world_with};
use crate::domain::made_tables::operands::service::file_cell;
use crate::domain::made_tables::reading::service::{cell_held, column_held};
use crate::domain::made_tables::sources::service::made;
use crate::domain::made_tables::types::ReadSource;
use crate::domain::steps::types::{MadeTableId, StepOperand};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::NodeExtent;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use std::borrow::Cow;

/// When the columns are read, after they were held.
#[derive(Clone, Copy, Debug)]
enum Read {
    /// At once.
    AtOnce,
    /// After a second table was made: the store changed under the held columns.
    AfterAnotherTable,
}

type Input = Read;
type Value = Cow<'static, str>;
type Failure = FilledError;

/// The relation's rows.
const ROWS: [[u64; 2]; 5] = [[1, 10], [2, 20], [1, 30], [3, 10], [1, 10]];

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_held_column_says_where_it_lies_and_the_file_column_is_one_cell"),
        word: Word::Succeeds,
        input: || Read::AtOnce,
        expect: Expect::Value(Cow::Borrowed(
            "Arena from 0 by 2 to 10: 1 2 1 3 1 0 | TableFile(MadeTableId(0)) from 0 by 0 to 1: F F F F F F | Arena from 1 by 2 to 10: 10 20 30 10 10 0 | Relation(RelationNumber(0), Short(1)) from 0 by 1 to its end: 10 20 30 10 10 0 | Nothing from 0 by 1 to 0: 0 0 0 0 0 0 | Nothing from 0 by 1 to 0: 0 0 0 0 0 0",
        )),
    },
    Case {
        name: CaseName("a_held_column_stays_good_while_the_store_changes"),
        word: Word::Succeeds,
        input: || Read::AfterAnotherTable,
        expect: Expect::Value(Cow::Borrowed(
            "Arena from 0 by 2 to 10: 1 2 1 3 1 0 | TableFile(MadeTableId(0)) from 0 by 0 to 1: F F F F F F | Arena from 1 by 2 to 10: 10 20 30 10 10 0 | Relation(RelationNumber(0), Short(1)) from 0 by 1 to its end: 10 20 30 10 10 0 | Nothing from 0 by 1 to 0: 0 0 0 0 0 0 | Nothing from 0 by 1 to 0: 0 0 0 0 0 0",
        )),
    },
];

/// The tables: the relation's first column, the file, its second column; and a second table of the first column.
fn tables() -> Vec<MadeTable> {
    vec![
        table(
            1,
            3,
            MadeFill::Rows(
                Box::new([pipeline(
                    vec![scan(0), read(FIRST, 0, 1), read(SECOND, 0, 2)],
                    [of(1), StepOperand::File, of(2)],
                )]),
                RepeatedRows::Kept,
            ),
        ),
        rows(1, pipeline(vec![scan(0), read(FIRST, 0, 1)], [of(1)])),
    ]
}

fn subject(when: Input) -> Result<Value, Failure> {
    let mut world = world_with(tables(), Vec::new(), &ROWS, Telling::Keyed)?;
    let (mut frame, looped) = framed(&mut world);
    let _first = made(&mut frame, looped, MadeTableId(0))?;
    let first = ReadSource::Made(MadeTableId(0));
    // Each column of the first table, one of the relation, one of a table not made, and one of the node table.
    let held = [
        column_held(frame.store, first, Short(0)),
        column_held(frame.store, first, Short(1)),
        column_held(frame.store, first, Short(2)),
        column_held(frame.store, ReadSource::Filled(RelationNumber(0)), Short(1)),
        column_held(frame.store, ReadSource::Made(MadeTableId(1)), Short(0)),
        column_held(frame.store, ReadSource::Nodes(NodeExtent::Own), Short(0)),
    ];
    if matches!(when, Read::AfterAnotherTable) {
        let _second = made(&mut frame, looped, MadeTableId(1))?;
    }
    let file = file_cell(FILE);
    let said: Vec<String> = held
        .iter()
        .map(|column| {
            // The five rows, and one past them: no cell of the table, whatever lies after it in the arena.
            let cells: Vec<String> = (0..6_u64)
                .map(|row| {
                    let cell = cell_held(frame.store, *column, CellWord(row));
                    if cell == file {
                        "F".to_owned()
                    } else {
                        cell.0.to_string()
                    }
                })
                .collect();
            format!(
                "{:?} from {} by {} to {}: {}",
                column.cells,
                column.first.0,
                column.stride.0,
                if column.end.0 == usize::MAX {
                    "its end".to_owned()
                } else {
                    column.end.0.to_string()
                },
                cells.join(" ")
            )
        })
        .collect();
    Ok(Cow::Owned(said.join(" | ")))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
