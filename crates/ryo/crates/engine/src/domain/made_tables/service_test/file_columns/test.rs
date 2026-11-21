//! The cases of the file's own column of a made table, each run through `made` on a pack the contract's encoding wrote. A
//! table keeps no cell for a column every one of its pipelines writes the file into, and every reader that asks for
//! that column is handed the file's cell: a read by a row's handle, a key of an index, a grouping, the seam. A case
//! says the rows a reader sees (`F` for the file's cell), and how many cells the table keeps a row.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::Short;
use crate::domain::filled::errors::FilledError;
use crate::domain::lowered::types::{FillInput, GroupFill, MadeFill, MadeTable, OrderedFill};
use crate::domain::made_tables::fixtures::packs::{
    FILE, FIRST, SECOND, of, operands, pipeline, read, rows, scan, table, word,
};
use crate::domain::made_tables::fixtures::{Telling, framed, shown, world_with};
use crate::domain::made_tables::operands::service::file_cell;
use crate::domain::made_tables::sources::service::{cell_of, made, table_of};
use crate::domain::made_tables::types::ReadSource;
use crate::domain::steps::stages::types::{FillStage, UseMode};
use crate::domain::steps::types::{
    ColumnList, MadeColumn, MadeTableId, MatchColumns, ReadColumn, Register, StepOperand, StepRows,
    TestOperation,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::plan_orders::types::GroupEnd;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use std::borrow::Cow;

/// What a case makes and reads.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// A table of a column, the file, a column: its rows as a reader of each column sees them.
    Kept,
    /// The same with repeats dropped, two of the relation's rows being alike.
    Dropped,
    /// A second table that scans the first and reads its file column and its last by the row's handle.
    ReadBack,
    /// A second table that probes the first by its file column and its first column, asked with the file.
    ProbeWithTheFile,
    /// The same asked with another file's cell.
    ProbeWithAnotherFile,
    /// A second table that scans the bucket of the first that the file names: a key of the file column alone.
    ProbeByTheFileAlone,
    /// A counted grouping by the file and a column.
    Counted,
    /// A sorted table whose rows hold the file.
    Sorted,
    /// The seam's `cell_of`, asked for the file column of the first table's second row.
    CellOf,
    /// A table of two pipelines, one of which writes a column read where the other writes the file.
    Mixed,
    /// The rows at the least end of each key's rows, of an input that holds the file.
    Ends,
    /// An ordered match of an input that holds the file with itself.
    Matched,
}

type Input = Asked;
type Value = Cow<'static, str>;
type Failure = FilledError;

/// The relation's rows.
const ROWS: [[u64; 2]; 5] = [[1, 10], [2, 20], [1, 30], [3, 10], [1, 10]];

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_table_keeps_no_cell_of_the_file_and_a_reader_of_its_rows_sees_the_file"),
        word: Word::Succeeds,
        input: || Asked::Kept,
        expect: Expect::Value(Cow::Borrowed(
            "1 F 10; 2 F 20; 1 F 30; 3 F 10; 1 F 10 | the first table keeps 2 cells a row",
        )),
    },
    Case {
        name: CaseName("repeats_are_dropped_by_the_cells_the_table_keeps"),
        word: Word::Succeeds,
        input: || Asked::Dropped,
        expect: Expect::Value(Cow::Borrowed(
            "1 F 10; 2 F 20; 1 F 30; 3 F 10 | the first table keeps 2 cells a row",
        )),
    },
    Case {
        name: CaseName("a_read_of_the_file_column_by_a_rows_handle_gives_the_file"),
        word: Word::Succeeds,
        input: || Asked::ReadBack,
        expect: Expect::Value(Cow::Borrowed(
            "F 10; F 20; F 30; F 10; F 10 | the first table keeps 2 cells a row",
        )),
    },
    Case {
        name: CaseName("a_key_that_holds_the_file_column_finds_its_rows_by_the_other_column"),
        word: Word::Succeeds,
        input: || Asked::ProbeWithTheFile,
        expect: Expect::Value(Cow::Borrowed(
            "1 F 10; 1 F 30; 1 F 10; 2 F 20; 1 F 10; 1 F 30; 1 F 10; 3 F 10; 1 F 10; 1 F 30; 1 F 10 | the first table keeps 2 cells a row",
        )),
    },
    Case {
        name: CaseName("a_key_that_asks_another_file_there_has_no_row"),
        word: Word::Succeeds,
        input: || Asked::ProbeWithAnotherFile,
        expect: Expect::Value(Cow::Borrowed(" | the first table keeps 2 cells a row")),
    },
    Case {
        name: CaseName("a_key_of_the_file_column_alone_holds_every_row"),
        word: Word::Succeeds,
        input: || Asked::ProbeByTheFileAlone,
        expect: Expect::Value(Cow::Borrowed(
            "1 F 10; 2 F 20; 1 F 30; 3 F 10 | the first table keeps 2 cells a row",
        )),
    },
    Case {
        name: CaseName("a_counted_grouping_by_the_file_and_a_column_holds_the_file_in_its_rows"),
        word: Word::Succeeds,
        input: || Asked::Counted,
        expect: Expect::Value(Cow::Borrowed(
            "F 1 3; F 2 1; F 3 1 | the first table keeps 2 cells a row",
        )),
    },
    Case {
        name: CaseName("a_sorted_table_hands_on_its_inputs_file_column"),
        word: Word::Succeeds,
        input: || Asked::Sorted,
        expect: Expect::Value(Cow::Borrowed(
            "F 1 10; F 1 10; F 1 30; F 2 20; F 3 10 | the first table keeps 2 cells a row",
        )),
    },
    Case {
        name: CaseName("the_seam_hands_the_file_for_a_cell_of_the_file_column"),
        word: Word::Succeeds,
        input: || Asked::CellOf,
        expect: Expect::Value(Cow::Borrowed("F | the first table keeps 2 cells a row")),
    },
    Case {
        name: CaseName("a_column_only_one_pipeline_writes_the_file_into_is_kept"),
        word: Word::Succeeds,
        input: || Asked::Mixed,
        expect: Expect::Value(Cow::Borrowed(
            "1 F 10; 2 F 20; 1 F 30; 3 F 10; 1 1 10; 2 2 20; 1 1 30; 3 3 10 | the first table keeps 3 cells a row",
        )),
    },
    Case {
        name: CaseName("a_groups_ends_hand_on_their_inputs_file_column"),
        word: Word::Succeeds,
        input: || Asked::Ends,
        expect: Expect::Value(Cow::Borrowed(
            "F 1 10; F 1 10; F 2 20; F 3 10 | the first table keeps 2 cells a row",
        )),
    },
    Case {
        name: CaseName("an_ordered_match_holds_both_rows_file_columns"),
        word: Word::Succeeds,
        input: || Asked::Matched,
        expect: Expect::Value(Cow::Borrowed(
            "F 1 10 F 1 30 | the first table keeps 4 cells a row",
        )),
    },
];

/// The first table: the relation's first column, the file, its second column.
fn with_the_file(repeats: RepeatedRows) -> MadeTable {
    table(
        1,
        3,
        MadeFill::Rows(
            Box::new([pipeline(
                vec![scan(0), read(FIRST, 0, 1), read(SECOND, 0, 2)],
                [of(1), StepOperand::File, of(2)],
            )]),
            repeats,
        ),
    )
}

/// A read of a column of the first table by a row's handle.
fn read_made(column: u16, row: u32, out: u32) -> FillStage {
    FillStage::Read {
        source: StepRows::Made(MadeTableId(0)),
        column: ReadColumn::Made(MadeColumn(column)),
        row: Register(row),
        out: Register(out),
    }
}

/// A second table that probes the first by the columns given with the values given, and writes the relation's first
/// column, the file column of the row found and its last column.
fn prober<const N: usize>(columns: [u16; N], values: [StepOperand; N]) -> MadeTable {
    rows(
        3,
        pipeline(
            vec![
                scan(0),
                read(FIRST, 0, 1),
                FillStage::Probe {
                    source: StepRows::Made(MadeTableId(0)),
                    columns: MatchColumns(
                        columns
                            .iter()
                            .map(|column| ReadColumn::Made(MadeColumn(*column)))
                            .collect(),
                    ),
                    values: operands(values),
                    range: None,
                    mode: UseMode::Rows,
                    out: Register(3),
                },
                read_made(1, 3, 4),
                read_made(2, 3, 5),
            ],
            [of(1), of(4), of(5)],
        ),
    )
}

/// The input of a grouping fill: the file, the relation's first column, its second; keyed by the first two.
fn input() -> FillInput {
    FillInput {
        pipelines: Box::new([pipeline(
            vec![scan(0), read(FIRST, 0, 1), read(SECOND, 0, 2)],
            [StepOperand::File, of(1), of(2)],
        )]),
        keys: ColumnList(Box::new([MadeColumn(0), MadeColumn(1)])),
        order: Some(MadeColumn(2)),
    }
}

/// The input of an ordered match: the same columns over the relation's rows but its last, which repeats the first, so
/// that no key holds an order twice.
fn distinct() -> FillInput {
    FillInput {
        pipelines: Box::new([pipeline(
            vec![
                scan(0),
                read(FIRST, 0, 1),
                read(SECOND, 0, 2),
                FillStage::Keep {
                    test: TestOperation::NotEqual,
                    left: of(0),
                    right: word(4),
                },
            ],
            [StepOperand::File, of(1), of(2)],
        )]),
        keys: ColumnList(Box::new([MadeColumn(0), MadeColumn(1)])),
        order: Some(MadeColumn(2)),
    }
}

/// The tables of a case; the last is the one made.
fn tables(asked: Asked) -> Vec<MadeTable> {
    let other = CellWord(file_cell(FileId(FILE.0.saturating_add(1))).0);
    match asked {
        Asked::Kept | Asked::CellOf => vec![with_the_file(RepeatedRows::Kept)],
        Asked::Dropped => vec![with_the_file(RepeatedRows::Dropped)],
        Asked::ReadBack => vec![
            with_the_file(RepeatedRows::Kept),
            rows(
                2,
                pipeline(
                    vec![
                        FillStage::Scan {
                            source: StepRows::Made(MadeTableId(0)),
                            columns: MatchColumns::default(),
                            values: operands([]),
                            out: Register(0),
                        },
                        read_made(1, 0, 1),
                        read_made(2, 0, 2),
                    ],
                    [of(1), of(2)],
                ),
            ),
        ],
        Asked::ProbeWithTheFile => vec![
            with_the_file(RepeatedRows::Kept),
            prober([1, 0], [StepOperand::File, of(1)]),
        ],
        Asked::ProbeWithAnotherFile => vec![
            with_the_file(RepeatedRows::Kept),
            prober([1, 0], [word(other.0), of(1)]),
        ],
        Asked::ProbeByTheFileAlone => vec![
            with_the_file(RepeatedRows::Dropped),
            rows(
                3,
                pipeline(
                    vec![
                        // A scan of the bucket of the first table that the file names.
                        FillStage::Scan {
                            source: StepRows::Made(MadeTableId(0)),
                            columns: MatchColumns(Box::new([ReadColumn::Made(MadeColumn(1))])),
                            values: operands([word(file_cell(FILE).0)]),
                            out: Register(0),
                        },
                        read_made(0, 0, 1),
                        read_made(1, 0, 2),
                        read_made(2, 0, 3),
                    ],
                    [of(1), of(2), of(3)],
                ),
            ),
        ],
        Asked::Counted => vec![table(
            1,
            3,
            MadeFill::Group(GroupFill::Count { distinct: None }, input()),
        )],
        Asked::Sorted => vec![table(1, 3, MadeFill::Ordered(OrderedFill::Sorted(input())))],
        Asked::Mixed => vec![table(
            1,
            3,
            MadeFill::Rows(
                Box::new([
                    pipeline(
                        vec![scan(0), read(FIRST, 0, 1), read(SECOND, 0, 2)],
                        [of(1), StepOperand::File, of(2)],
                    ),
                    pipeline(
                        vec![scan(0), read(FIRST, 0, 1), read(SECOND, 0, 2)],
                        [of(1), of(1), of(2)],
                    ),
                ]),
                RepeatedRows::Dropped,
            ),
        )],
        Asked::Ends => vec![table(
            1,
            3,
            MadeFill::Group(GroupFill::Ends(GroupEnd::Least), input()),
        )],
        Asked::Matched => vec![table(
            1,
            6,
            MadeFill::Ordered(OrderedFill::Match(distinct(), distinct())),
        )],
    }
}

fn subject(asked: Input) -> Result<Value, Failure> {
    let made_tables = tables(asked);
    let last = MadeTableId(u32::try_from(made_tables.len().saturating_sub(1)).unwrap_or(0));
    let mut world = world_with(made_tables, Vec::new(), &ROWS, Telling::Keyed)?;
    let (mut frame, looped) = framed(&mut world);
    let _now = made(&mut frame, looped, last)?;
    let file = file_cell(FILE).0.to_string();
    let seen = if matches!(asked, Asked::CellOf) {
        // The second row of the table, by its handle: its number.
        let cell = cell_of(&mut frame, ReadSource::Made(last), Short(1), CellWord(1));
        cell.0.to_string()
    } else {
        shown(frame.store, last)
    };
    // The file's cell is shown as `F`: it is a long number.
    let said: Vec<String> = seen
        .split(' ')
        .map(|cell| {
            let bare = cell.trim_end_matches(';');
            if bare == file {
                cell.replace(bare, "F")
            } else {
                cell.to_owned()
            }
        })
        .collect();
    let kept = table_of(frame.store, MadeTableId(0)).map_or(0, |slot| slot.rows.width.0);
    Ok(Cow::Owned(format!(
        "{} | the first table keeps {kept} cells a row",
        said.join(" ")
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
