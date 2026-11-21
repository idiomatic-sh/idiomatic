//! The cases of a table's fill, each run through `made` on a pack the contract's encoding wrote: one case for each fill
//! the fill side runs, the record read back as the encoding wrote it, and the fills it does not run yet.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::FillCode;
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::types::UnbuiltFill;
use crate::domain::lowered::types::{
    ConstantRows, FillInput, GroupFill, MadeFill, MadeTable, OrderedFill,
};
use crate::domain::made_tables::fixtures::packs::{FIRST, SECOND, TOLD};
use crate::domain::made_tables::fixtures::packs::{of, operands, pipeline, table};
use crate::domain::made_tables::fixtures::{Telling, framed, shown, world_with};
use crate::domain::made_tables::sources::service::{made, table_of};
use crate::domain::steps::stages::types::{FillStage, Pipeline, UseMode};
use crate::domain::steps::types::{
    ColumnList, MadeColumn, MadeTableId, MatchColumns, ReadColumn, Register, StepOperand, StepRows,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use std::borrow::Cow;

/// The fill a case runs.
#[derive(Clone, Copy, Debug)]
enum Filled {
    /// A definition's rows, repeats dropped.
    RowsDropped,
    /// A definition's rows from two pipelines, repeats kept.
    TwoPipelines,
    /// How many rows hold each key.
    Count,
    /// How many different cells of a column each key holds.
    CountDistinct,
    /// The rows of each key together, ascending by the order column.
    Sorted,
    /// Rows written in the code.
    Constant,
    /// A pipeline whose scan has no row: nothing else of it is read.
    EmptyScan,
    /// A digested grouping, which the fill side does not run yet.
    Digest,
}

type Input = Filled;
type Value = Cow<'static, str>;
type Failure = FilledError;

/// The relation's rows: the first and the last are the same.
const ROWS: [[u64; 2]; 5] = [[1, 10], [2, 20], [1, 30], [3, 10], [1, 10]];

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_rows_fill_that_drops_repeats_holds_each_row_once"),
        word: Word::Succeeds,
        input: || Filled::RowsDropped,
        expect: Expect::Value(Cow::Borrowed(
            "1 10; 2 20; 1 30; 3 10 | orders [] | read 20 given 15 made 4 | reached [Relation(RelationNumber(0))] | the language was asked 1 times",
        )),
    },
    Case {
        name: CaseName("a_rows_fill_of_two_pipelines_holds_the_rows_of_both"),
        word: Word::Succeeds,
        input: || Filled::TwoPipelines,
        expect: Expect::Value(Cow::Borrowed(
            "1; 2; 1; 3; 1; 10; 20; 30; 10; 10 | orders [] | read 40 given 30 made 10 | reached [Relation(RelationNumber(0)), Relation(RelationNumber(0))] | the language was asked 1 times",
        )),
    },
    Case {
        name: CaseName("a_counted_grouping_holds_each_key_and_how_many_rows_hold_it"),
        word: Word::Succeeds,
        input: || Filled::Count,
        expect: Expect::Value(Cow::Borrowed(
            "1 3; 2 1; 3 1 | orders [] | read 25 given 18 made 3 | reached [Relation(RelationNumber(0))] | the language was asked 1 times",
        )),
    },
    Case {
        name: CaseName("a_distinct_count_counts_the_different_cells_of_its_column"),
        word: Word::Succeeds,
        input: || Filled::CountDistinct,
        expect: Expect::Value(Cow::Borrowed(
            "1 2; 2 1; 3 1 | orders [] | read 25 given 18 made 3 | reached [Relation(RelationNumber(0))] | the language was asked 1 times",
        )),
    },
    Case {
        name: CaseName("a_sorted_table_holds_each_keys_rows_together_and_ascending"),
        word: Word::Succeeds,
        input: || Filled::Sorted,
        expect: Expect::Value(Cow::Borrowed(
            "1 10; 1 10; 1 30; 2 20; 3 10 | orders [10, 10, 30, 20, 10] | read 25 given 20 made 5 | reached [Relation(RelationNumber(0))] | the language was asked 1 times",
        )),
    },
    Case {
        name: CaseName("a_constant_table_holds_the_rows_written"),
        word: Word::Succeeds,
        input: || Filled::Constant,
        expect: Expect::Value(Cow::Borrowed(
            "5 40; 6 41 | orders [] | read 0 given 0 made 2 | reached [] | the language was asked 0 times",
        )),
    },
    Case {
        name: CaseName("a_pipeline_whose_scan_has_no_row_reads_nothing_else"),
        word: Word::Succeeds,
        input: || Filled::EmptyScan,
        expect: Expect::Value(Cow::Borrowed(
            " | orders [] | read 0 given 0 made 0 | reached [Table(MadeTableId(0))] | the language was asked 0 times",
        )),
    },
    Case {
        name: CaseName("a_fill_the_fill_side_does_not_run_yet_is_said_so"),
        word: Word::Refuses,
        input: || Filled::Digest,
        expect: Expect::Error(FilledError::Unbuilt {
            unbuilt: UnbuiltFill::Fill(FillCode(2)),
        }),
    },
];

/// A pipeline that scans the relation and writes the cells of the columns named: 1 the first, 2 the second.
fn told<const N: usize>(row: [StepOperand; N]) -> Pipeline {
    let read = |column, out| FillStage::Read {
        source: StepRows::Filled(TOLD),
        column: ReadColumn::Filled(column),
        row: Register(0),
        out: Register(out),
    };
    pipeline(
        vec![
            FillStage::Scan {
                source: StepRows::Filled(TOLD),
                columns: MatchColumns::default(),
                values: operands([]),
                out: Register(0),
            },
            read(FIRST, 1),
            read(SECOND, 2),
        ],
        row,
    )
}

/// The input of a grouping fill: the relation's two columns, keyed by the first.
fn input(order: Option<MadeColumn>) -> FillInput {
    FillInput {
        pipelines: Box::new([told([of(1), of(2)])]),
        keys: ColumnList(Box::new([MadeColumn(0)])),
        order,
    }
}

/// The tables of a case; the last is the one made.
fn tables(filled: Filled) -> Vec<MadeTable> {
    match filled {
        Filled::RowsDropped => vec![table(
            1,
            2,
            MadeFill::Rows(Box::new([told([of(1), of(2)])]), RepeatedRows::Dropped),
        )],
        Filled::TwoPipelines => vec![table(
            1,
            1,
            MadeFill::Rows(Box::new([told([of(1)]), told([of(2)])]), RepeatedRows::Kept),
        )],
        Filled::Count => vec![table(
            1,
            2,
            MadeFill::Group(GroupFill::Count { distinct: None }, input(None)),
        )],
        Filled::CountDistinct => vec![table(
            1,
            2,
            MadeFill::Group(
                GroupFill::Count {
                    distinct: Some(MadeColumn(1)),
                },
                input(None),
            ),
        )],
        Filled::Sorted => vec![table(
            1,
            2,
            MadeFill::Ordered(OrderedFill::Sorted(input(Some(MadeColumn(1))))),
        )],
        Filled::Constant => vec![table(
            1,
            2,
            MadeFill::Constant(ConstantRows(vec![
                Box::new([CellWord(5), CellWord(40)]),
                Box::new([CellWord(6), CellWord(41)]),
            ])),
        )],
        Filled::EmptyScan => vec![
            table(2, 1, MadeFill::Constant(ConstantRows(Vec::new()))),
            table(
                1,
                1,
                MadeFill::Rows(
                    Box::new([pipeline(
                        vec![
                            FillStage::Scan {
                                source: StepRows::Made(MadeTableId(0)),
                                columns: MatchColumns::default(),
                                values: operands([]),
                                out: Register(0),
                            },
                            FillStage::Read {
                                source: StepRows::Made(MadeTableId(0)),
                                column: ReadColumn::Made(MadeColumn(0)),
                                row: Register(0),
                                out: Register(1),
                            },
                            FillStage::Probe {
                                source: StepRows::Filled(TOLD),
                                columns: MatchColumns(Box::new([ReadColumn::Filled(FIRST)])),
                                values: operands([of(1)]),
                                range: None,
                                mode: UseMode::Present,
                                out: Register(2),
                            },
                        ],
                        [of(1)],
                    )]),
                    RepeatedRows::Kept,
                ),
            ),
        ],
        Filled::Digest => vec![table(
            1,
            2,
            MadeFill::Group(
                GroupFill::Digest {
                    over: MadeColumn(1),
                },
                input(None),
            ),
        )],
    }
}

fn subject(filled: Input) -> Result<Value, Failure> {
    let made_tables = tables(filled);
    let last = MadeTableId(u32::try_from(made_tables.len().saturating_sub(1)).unwrap_or(0));
    let mut world = world_with(made_tables, Vec::new(), &ROWS, Telling::Keyed)?;
    let (mut frame, looped) = framed(&mut world);
    let _now = made(&mut frame, looped, last)?;
    let said = shown(frame.store, last);
    let slot = table_of(frame.store, last);
    let work = slot.map(|slot| slot.work).unwrap_or_default();
    let orders: Vec<i64> = slot
        .map(|slot| slot.rows.orders.iter().map(|order| order.0).collect())
        .unwrap_or_default();
    let reached = slot.map(|slot| slot.reached.clone()).unwrap_or_default();
    drop(frame);
    Ok(Cow::Owned(format!(
        "{said} | orders {orders:?} | read {} given {} made {} | reached {reached:?} | the language was asked {} times",
        work.read.0,
        work.given.0,
        work.made.0,
        world.told.asked.len()
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
