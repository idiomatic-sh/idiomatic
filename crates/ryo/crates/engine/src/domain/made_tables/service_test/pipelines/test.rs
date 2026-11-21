//! The cases of a pipeline's stages over a relation and made tables, each run through `made` on a pack the contract's
//! encoding wrote: one case for each kind of stage and each mode. A case says its table's rows and the fill's work.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::filled::errors::FilledError;
use crate::domain::lowered::types::{FillInput, MadeFill, MadeTable, OrderedFill};
use crate::domain::made_tables::fixtures::packs::{FIRST, SECOND, TOLD};
use crate::domain::made_tables::fixtures::packs::{
    constants, definition, of, operands, pipeline, probe, read, rows, scan, scan_made, table, used,
    word,
};
use crate::domain::made_tables::fixtures::{Telling, framed, shown, world_with};
use crate::domain::made_tables::sources::service::{made, table_of};
use crate::domain::steps::stages::types::{FillStage, ProbeRange, UseMode};
use crate::domain::steps::types::{
    BoundKind, ColumnList, MadeColumn, MadeTableId, MatchColumns, RangeBound, ReadColumn, Register,
    StepOperand, StepRows, TestOperation,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_values::domain::plan_orders::types::OrderBound;
use std::borrow::Cow;

/// The stage a case runs.
#[derive(Clone, Copy, Debug)]
enum Staged {
    /// A scan of the relation, and a read of its two columns.
    ScanAndRead,
    /// A scan of the rows of one constant key.
    ScanBucket,
    /// A keep of the rows whose second cell is below 30.
    KeepLess,
    /// A keep whose comparison holds for no row.
    KeepNone,
    /// A keep of the rows whose first cell is in a set.
    KeepIn,
    /// A keep of the rows whose first cell is not in a set.
    KeepNotIn,
    /// A probe that gives the relation's rows of each row's key, beside a column read before it.
    ProbeRows,
    /// A probe that keeps the rows whose key the relation holds.
    ProbePresent,
    /// A probe that keeps the rows whose key the relation does not hold.
    ProbeAbsent,
    /// A probe held to a range: above the low bound, up to and with the high.
    ProbeRange,
    /// A product with every row of the relation.
    Product,
    /// A use that collects a pattern's rows.
    UseRows,
    /// A use that keeps the rows whose key's pattern has a row.
    UsePresent,
    /// A use that keeps the rows whose key's pattern has none.
    UseAbsent,
    /// The latest row of a key at or before a bound, in a sorted table.
    Latest,
}

type Input = Staged;
type Value = Cow<'static, str>;
type Failure = FilledError;

/// The relation's rows.
const ROWS: [[u64; 2]; 5] = [[1, 10], [2, 20], [1, 30], [3, 10], [1, 40]];

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_scan_hands_every_row_and_a_read_its_cells"),
        word: Word::Succeeds,
        input: || Staged::ScanAndRead,
        expect: Expect::Value(Cow::Borrowed(
            "1 10; 2 20; 1 30; 3 10; 1 40 | read 20 given 15 made 5",
        )),
    },
    Case {
        name: CaseName("a_scan_of_a_bucket_hands_the_rows_of_its_key"),
        word: Word::Succeeds,
        input: || Staged::ScanBucket,
        expect: Expect::Value(Cow::Borrowed("10; 30; 40 | read 9 given 6 made 3")),
    },
    Case {
        name: CaseName("a_keep_holds_the_rows_its_comparison_holds_for"),
        word: Word::Succeeds,
        input: || Staged::KeepLess,
        expect: Expect::Value(Cow::Borrowed("1 10; 2 20; 3 10 | read 23 given 18 made 3")),
    },
    Case {
        name: CaseName("a_keep_that_holds_for_no_row_leaves_none"),
        word: Word::Succeeds,
        input: || Staged::KeepNone,
        expect: Expect::Value(Cow::Borrowed(" | read 20 given 15 made 0")),
    },
    Case {
        name: CaseName("a_keep_of_a_set_holds_the_rows_whose_cell_is_in_it"),
        word: Word::Succeeds,
        input: || Staged::KeepIn,
        expect: Expect::Value(Cow::Borrowed("2 20; 3 10 | read 22 given 17 made 2")),
    },
    Case {
        name: CaseName("a_keep_of_a_set_negated_holds_the_others"),
        word: Word::Succeeds,
        input: || Staged::KeepNotIn,
        expect: Expect::Value(Cow::Borrowed("1 10; 1 30; 1 40 | read 23 given 18 made 3")),
    },
    Case {
        name: CaseName("a_probe_gives_each_rows_matches_and_carries_its_columns"),
        word: Word::Succeeds,
        input: || Staged::ProbeRows,
        expect: Expect::Value(Cow::Borrowed(
            "1 10 10; 1 10 30; 1 10 40; 2 20 20; 1 30 10; 1 30 30; 1 30 40; 3 10 10; 1 40 10; 1 40 30; 1 40 40 | read 42 given 37 made 11",
        )),
    },
    Case {
        name: CaseName("a_probe_for_presence_keeps_the_rows_whose_key_is_held"),
        word: Word::Succeeds,
        input: || Staged::ProbePresent,
        expect: Expect::Value(Cow::Borrowed("1; 3 | read 14 given 10 made 2")),
    },
    Case {
        name: CaseName("a_probe_for_absence_keeps_the_rows_whose_key_is_not_held"),
        word: Word::Succeeds,
        input: || Staged::ProbeAbsent,
        expect: Expect::Value(Cow::Borrowed("4; 9 | read 14 given 10 made 2")),
    },
    Case {
        name: CaseName("a_probe_with_a_range_gives_the_rows_between_its_bounds"),
        word: Word::Succeeds,
        input: || Staged::ProbeRange,
        expect: Expect::Value(Cow::Borrowed("1 30; 1 40; 2 20 | read 12 given 10 made 3")),
    },
    Case {
        name: CaseName("a_product_pairs_each_row_with_every_row_of_its_source"),
        word: Word::Succeeds,
        input: || Staged::Product,
        expect: Expect::Value(Cow::Borrowed(
            "7 10; 7 20; 7 30; 7 10; 7 40; 8 10; 8 20; 8 30; 8 10; 8 40 | read 26 given 24 made 10 | a product of 2 rows with 5",
        )),
    },
    Case {
        name: CaseName("a_use_asks_the_loop_once_for_each_distinct_key_and_gives_its_rows"),
        word: Word::Succeeds,
        input: || Staged::UseRows,
        expect: Expect::Value(Cow::Borrowed(
            "4 400; 5 500; 5 501; 4 400; 5 500; 5 501 | read 21 given 16 made 6 | asked [[3], [4], [5]]",
        )),
    },
    Case {
        name: CaseName("a_use_for_presence_keeps_the_rows_whose_pattern_has_a_row"),
        word: Word::Succeeds,
        input: || Staged::UsePresent,
        expect: Expect::Value(Cow::Borrowed(
            "4; 5; 4 | read 18 given 13 made 3 | asked [[3], [4], [5], [6]]",
        )),
    },
    Case {
        name: CaseName("a_use_for_absence_keeps_the_rows_whose_pattern_has_none"),
        word: Word::Succeeds,
        input: || Staged::UseAbsent,
        expect: Expect::Value(Cow::Borrowed(
            "3; 6 | read 17 given 12 made 2 | asked [[3], [4], [5], [6]]",
        )),
    },
    Case {
        name: CaseName("a_latest_gives_the_keys_row_nearest_at_or_before_its_bound"),
        word: Word::Succeeds,
        input: || Staged::Latest,
        expect: Expect::Value(Cow::Borrowed("1 30; 2 20; 3 10 | read 18 given 14 made 3")),
    },
];

/// The tables of a case; the last is the one made.
fn tables(staged: Staged) -> Vec<MadeTable> {
    match staged {
        Staged::ScanAndRead => vec![rows(
            2,
            pipeline(
                vec![scan(0), read(FIRST, 0, 1), read(SECOND, 0, 2)],
                [of(1), of(2)],
            ),
        )],
        Staged::ScanBucket => vec![rows(
            1,
            pipeline(
                vec![
                    FillStage::Scan {
                        source: StepRows::Filled(TOLD),
                        columns: MatchColumns(Box::new([ReadColumn::Filled(FIRST)])),
                        values: operands([word(1)]),
                        out: Register(0),
                    },
                    read(SECOND, 0, 2),
                ],
                [of(2)],
            ),
        )],
        Staged::KeepLess => vec![rows(
            2,
            pipeline(
                vec![
                    scan(0),
                    read(FIRST, 0, 1),
                    read(SECOND, 0, 2),
                    FillStage::Keep {
                        test: TestOperation::Less,
                        left: of(2),
                        right: word(30),
                    },
                ],
                [of(1), of(2)],
            ),
        )],
        Staged::KeepNone => vec![rows(
            2,
            pipeline(
                vec![
                    scan(0),
                    read(FIRST, 0, 1),
                    read(SECOND, 0, 2),
                    FillStage::Keep {
                        test: TestOperation::Less,
                        left: of(2),
                        right: word(5),
                    },
                ],
                [of(1), of(2)],
            ),
        )],
        Staged::KeepIn | Staged::KeepNotIn => vec![rows(
            2,
            pipeline(
                vec![
                    scan(0),
                    read(FIRST, 0, 1),
                    read(SECOND, 0, 2),
                    FillStage::Keep {
                        test: if matches!(staged, Staged::KeepIn) {
                            TestOperation::In
                        } else {
                            TestOperation::NotIn
                        },
                        left: of(1),
                        right: StepOperand::Words(Box::new([CellWord(3), CellWord(2)])),
                    },
                ],
                [of(1), of(2)],
            ),
        )],
        Staged::ProbeRows => vec![rows(
            3,
            pipeline(
                vec![
                    scan(0),
                    read(FIRST, 0, 1),
                    read(SECOND, 0, 2),
                    probe(1, None, UseMode::Rows, 3),
                    read(SECOND, 3, 4),
                ],
                [of(1), of(2), of(4)],
            ),
        )],
        Staged::ProbePresent | Staged::ProbeAbsent => {
            let mode = if matches!(staged, Staged::ProbePresent) {
                UseMode::Present
            } else {
                UseMode::Absent
            };
            let mut all = Vec::from(scan_made(0, 0, 1));
            all.push(probe(1, None, mode, 3));
            vec![constants(&[1, 4, 3, 9]), rows(1, pipeline(all, [of(1)]))]
        }
        Staged::ProbeRange => {
            let bound = |cell: u64, end: BoundKind| RangeBound {
                value: word(cell),
                end,
            };
            let range = ProbeRange {
                column: ReadColumn::Filled(SECOND),
                low: bound(10, BoundKind::Strict),
                high: bound(40, BoundKind::Inclusive),
            };
            let mut all = Vec::from(scan_made(0, 0, 1));
            all.push(probe(1, Some(range), UseMode::Rows, 3));
            all.push(read(SECOND, 3, 4));
            vec![constants(&[1, 2]), rows(2, pipeline(all, [of(1), of(4)]))]
        }
        Staged::Product => {
            let mut all = Vec::from(scan_made(0, 0, 1));
            all.push(FillStage::Product {
                source: StepRows::Filled(TOLD),
                out: Register(3),
            });
            all.push(read(SECOND, 3, 4));
            vec![constants(&[7, 8]), rows(2, pipeline(all, [of(1), of(4)]))]
        }
        Staged::UseRows => {
            let mut all = Vec::from(scan_made(0, 0, 1));
            all.push(used(1, UseMode::Rows, &[5]));
            vec![
                constants(&[4, 5, 3, 4, 5]),
                rows(2, pipeline(all, [of(1), of(5)])),
            ]
        }
        Staged::UsePresent | Staged::UseAbsent => {
            let mode = if matches!(staged, Staged::UsePresent) {
                UseMode::Present
            } else {
                UseMode::Absent
            };
            let mut all = Vec::from(scan_made(0, 0, 1));
            all.push(used(1, mode, &[]));
            vec![constants(&[4, 5, 3, 4, 6]), rows(1, pipeline(all, [of(1)]))]
        }
        Staged::Latest => {
            let sorted = table(
                92,
                2,
                MadeFill::Ordered(OrderedFill::Sorted(FillInput {
                    pipelines: Box::new([pipeline(
                        vec![scan(10), read(FIRST, 10, 11), read(SECOND, 10, 12)],
                        [of(11), of(12)],
                    )]),
                    keys: ColumnList(Box::new([MadeColumn(0)])),
                    order: Some(MadeColumn(1)),
                })),
            );
            let mut all = Vec::from(scan_made(0, 0, 1));
            all.push(FillStage::Latest {
                table: MadeTableId(1),
                key: operands([of(1)]),
                bound: word(30),
                side: OrderBound::AtOrBefore,
                out: Register(3),
            });
            all.push(FillStage::Read {
                source: StepRows::Made(MadeTableId(1)),
                column: ReadColumn::Made(MadeColumn(1)),
                row: Register(3),
                out: Register(4),
            });
            vec![
                constants(&[1, 2, 3, 5]),
                sorted,
                rows(2, pipeline(all, [of(1), of(4)])),
            ]
        }
    }
}

fn subject(staged: Input) -> Result<Value, Failure> {
    let made_tables = tables(staged);
    let last = MadeTableId(u32::try_from(made_tables.len().saturating_sub(1)).unwrap_or(0));
    let mut world = world_with(made_tables, vec![definition()], &ROWS, Telling::Keyed)?;
    let (mut frame, looped) = framed(&mut world);
    let _now = made(&mut frame, looped, last)?;
    let said = shown(frame.store, last);
    let work = table_of(frame.store, last)
        .map(|slot| slot.work)
        .unwrap_or_default();
    let keys = if looped.keys.is_empty() {
        String::new()
    } else {
        format!(" | asked {:?}", looped.keys)
    };
    let pairs = if work.left.0 == 0 {
        String::new()
    } else {
        format!(" | a product of {} rows with {}", work.left.0, work.right.0)
    };
    Ok(Cow::Owned(format!(
        "{said} | read {} given {} made {}{pairs}{keys}",
        work.read.0, work.given.0, work.made.0
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
