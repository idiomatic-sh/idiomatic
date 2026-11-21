//! The cases of what a worker's store keeps for its tables, each run through `made` on a pack the contract's encoding
//! wrote, over several files one after the other. A case COUNTS, after each file: the cells of the file's tables in
//! use, the cells the store holds room for (the one arena of a file's tables), and what the tables keep of their own
//! (a list of cells, a set of rows held), which must be nothing. The bound: a worker holds room for the most ONE file's
//! tables needed, never, table by table, the most each table ever needed. `kept` names every field of the store, so a
//! structure added to it and not counted here does not compile.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::filled::errors::FilledError;
use crate::domain::lowered::types::{FillInput, GroupFill, MadeFill, MadeTable};
use crate::domain::made_tables::columns::types::CellsLie;
use crate::domain::made_tables::fixtures::packs::{
    FIRST, SECOND, keep_equal, of, pipeline, read, rows, scan, table,
};
use crate::domain::made_tables::fixtures::{Telling, framed, world_with};
use crate::domain::made_tables::service::clear_rows;
use crate::domain::made_tables::sources::service::{made, next_file};
use crate::domain::made_tables::types::{FillStore, TableRows};
use crate::domain::made_tables::writing::service::{settled, written};
use crate::domain::stages::types::BatchValue;
use crate::domain::steps::types::{ColumnList, MadeColumn, MadeTableId};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use std::borrow::Cow;

/// The files a case scans, and its tables.
#[derive(Clone, Copy, Debug)]
enum Scanned {
    /// A file of a thousand rows, then one of five: a table of the rows, one with repeats dropped, a count.
    LargeThenSmall,
    /// One table's rows written at the arena's end in two parts, another table's cells written between them, as when
    /// a table is made while another's fill runs: the first table's cells, where they start, and the arena's length.
    WrittenAround,
    /// Three files of a thousand rows, each all of one key, and three tables that each keep one key's rows: every
    /// table is large in a file of its own.
    EachTableLargeInItsOwnFile,
}

type Input = Scanned;
type Value = Cow<'static, str>;
type Failure = FilledError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("after_a_small_file_the_store_holds_no_more_than_the_large_one_needed"),
        word: Word::Succeeds,
        input: || Scanned::LargeThenSmall,
        expect: Expect::Value(Cow::Borrowed(
            "rows [1000, 1000, 50]: arena 4100 in use, room for 8000; the tables' own lists 0, their own sets 0; lent: 1 lists, 1 sets, 1 inputs | rows [5, 5, 5]: arena 30 in use, room for 8000; the tables' own lists 0, their own sets 0; lent: 1 lists, 1 sets, 1 inputs",
        )),
    },
    Case {
        name: CaseName("tables_each_large_in_another_file_share_one_files_room"),
        word: Word::Succeeds,
        input: || Scanned::EachTableLargeInItsOwnFile,
        expect: Expect::Value(Cow::Borrowed(
            "rows [1000, 0, 0]: arena 2000 in use, room for 2000; the tables' own lists 0, their own sets 0; lent: 0 lists, 1 sets, 0 inputs | rows [0, 1000, 0]: arena 2000 in use, room for 2000; the tables' own lists 0, their own sets 0; lent: 0 lists, 1 sets, 0 inputs | rows [0, 0, 1000]: arena 2000 in use, room for 2000; the tables' own lists 0, their own sets 0; lent: 0 lists, 1 sets, 0 inputs",
        )),
    },
    Case {
        name: CaseName("a_table_written_around_another_is_moved_to_the_arenas_end_whole"),
        word: Word::Succeeds,
        input: || Scanned::WrittenAround,
        expect: Expect::Value(Cow::Borrowed(
            "3 rows from 7: [1, 10, 2, 20, 3, 30]; the arena holds 13 cells",
        )),
    },
];

/// What the store keeps for its tables: the cells in use and the cells it holds room for in the arena, and what the
/// tables and the lent lists keep of their own. Every field of the store is named: a new one must be counted or
/// said not to hold a table's rows.
fn kept(store: &FillStore) -> String {
    let FillStore {
        stamp: _,
        relations: _,
        indexes: _,
        tables,
        room: _,
        rooms: _,
        inputs,
        cells: _,
        writes: _,
        table_cells,
        own_cells,
        running: _,
        seens,
        verdicts: _,
        asked: _,
        asked_cells: _,
    } = store;
    let lists: usize = tables.iter().map(|slot| slot.rows.cells.capacity()).sum();
    let sets: usize = tables.iter().map(|slot| slot.rows.seen.capacity()).sum();
    format!(
        "arena {} in use, room for {}; the tables' own lists {lists}, their own sets {sets}; lent: {} lists, {} sets, {} inputs",
        table_cells.len(),
        table_cells.capacity(),
        own_cells.len(),
        seens.len(),
        inputs.len()
    )
}

/// Two rows of a table of two columns, three cells of another table, then one more row of the first.
fn written_around() -> String {
    let mut arena: Vec<CellWord> = Vec::new();
    let mut first = TableRows::default();
    clear_rows(&mut first, Count(2));
    first.lies = CellsLie::Arena(Count(0));
    let column = |cells: &[u64]| cells.iter().map(|cell| CellWord(*cell)).collect::<Vec<_>>();
    let (one, two) = (column(&[1, 2]), column(&[10, 20]));
    let _start = settled(&mut arena, &mut first);
    written(
        &mut arena,
        &mut first,
        &[BatchValue::Column(&one), BatchValue::Column(&two)],
        Count(2),
    );
    arena.extend(column(&[7, 7, 7]));
    let (three, four) = (column(&[3]), column(&[30]));
    let start = settled(&mut arena, &mut first);
    written(
        &mut arena,
        &mut first,
        &[BatchValue::Column(&three), BatchValue::Column(&four)],
        Count(1),
    );
    let held = arena.get(start.0..).unwrap_or_default();
    format!(
        "{} rows from {}: {:?}; the arena holds {} cells",
        first.count.0,
        start.0,
        held.iter().map(|cell| cell.0).collect::<Vec<_>>(),
        arena.len()
    )
}

/// The tables of a case.
fn tables(scanned: Scanned) -> Vec<MadeTable> {
    let both = || vec![scan(0), read(FIRST, 0, 1), read(SECOND, 0, 2)];
    match scanned {
        Scanned::LargeThenSmall => vec![
            rows(2, pipeline(both(), [of(1), of(2)])),
            table(
                1,
                2,
                MadeFill::Rows(
                    Box::new([pipeline(both(), [of(1), of(2)])]),
                    RepeatedRows::Dropped,
                ),
            ),
            table(
                2,
                2,
                MadeFill::Group(
                    GroupFill::Count { distinct: None },
                    FillInput {
                        pipelines: Box::new([pipeline(both(), [of(1), of(2)])]),
                        keys: ColumnList(Box::new([MadeColumn(0)])),
                        order: None,
                    },
                ),
            ),
        ],
        Scanned::WrittenAround => Vec::new(),
        Scanned::EachTableLargeInItsOwnFile => (1..=3_u64)
            .map(|key| {
                let mut all = both();
                all.push(keep_equal(1, key));
                rows(2, pipeline(all, [of(1), of(2)]))
            })
            .collect(),
    }
}

/// The relation's rows in a case's file: a first cell and a second that differs from row to row.
fn file_rows(scanned: Scanned, file: u64) -> Vec<[u64; 2]> {
    match scanned {
        Scanned::LargeThenSmall => {
            let count = if file == 0 { 1000 } else { 5 };
            (0..count).map(|row| [row % 50, row]).collect()
        }
        Scanned::EachTableLargeInItsOwnFile => {
            (0..1000).map(|row| [file.saturating_add(1), row]).collect()
        }
        Scanned::WrittenAround => Vec::new(),
    }
}

fn subject(scanned: Input) -> Result<Value, Failure> {
    if matches!(scanned, Scanned::WrittenAround) {
        return Ok(Cow::Owned(written_around()));
    }
    let made_tables = tables(scanned);
    let count = u32::try_from(made_tables.len()).unwrap_or(0);
    let mut world = world_with(made_tables, Vec::new(), &[], Telling::Keyed)?;
    let files = match scanned {
        Scanned::LargeThenSmall => 2,
        Scanned::EachTableLargeInItsOwnFile => 3,
        Scanned::WrittenAround => 0,
    };
    let mut said = Vec::new();
    for file in 0..files {
        world.told.rows = file_rows(scanned, file);
        next_file(&mut world.store);
        let mut made_rows = Vec::new();
        {
            let (mut frame, looped) = framed(&mut world);
            for table in 0..count {
                let _now = made(&mut frame, looped, MadeTableId(table))?;
                made_rows.push(
                    crate::domain::made_tables::sources::service::table_of(
                        frame.store,
                        MadeTableId(table),
                    )
                    .map_or(0, |slot| slot.rows.count.0),
                );
            }
        }
        said.push(format!("rows {made_rows:?}: {}", kept(&world.store)));
    }
    Ok(Cow::Owned(said.join(" | ")))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
