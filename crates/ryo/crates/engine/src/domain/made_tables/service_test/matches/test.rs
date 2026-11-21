//! The cases of the fills that pick and pair, each run through `made` on a pack the contract's encoding wrote:
//! `Group(Ends)`, the rows at one end of each key's rows, and `Ordered(Match)`, two inputs paired within each key, first
//! in, first out.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::filled::errors::FilledError;
use crate::domain::lowered::types::{
    ConstantRows, FillInput, GroupFill, MadeFill, MadeTable, OrderedFill,
};
use crate::domain::made_tables::fixtures::packs::{
    FIRST, SECOND, TOLD, of, operands, pipeline, pipeline_of, table,
};
use crate::domain::made_tables::fixtures::{Telling, framed, shown, world_with};
use crate::domain::made_tables::sources::service::{made, table_of};
use crate::domain::steps::stages::types::FillStage;
use crate::domain::steps::types::{
    ColumnList, MadeColumn, MadeTableId, MatchColumns, ReadColumn, Register, StepRows,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_values::domain::plan_orders::types::GroupEnd;
use std::borrow::Cow;

/// The fill a case runs.
#[derive(Clone, Copy, Debug)]
enum Filled {
    /// The rows at the greatest end of each key's rows.
    EndsGreatest,
    /// The rows at the least end of each key's rows.
    EndsLeast,
    /// An ordered match of one key: the contract's stated rows.
    MatchOneKey,
    /// An ordered match of several keys, one of them on one side alone.
    MatchKeys,
    /// An ordered match one of whose inputs holds a key's order twice.
    MatchDuplicate,
}

type Input = Filled;
type Value = Cow<'static, str>;
type Failure = FilledError;

/// The relation's rows: the first and the last are the same.
const ROWS: [[u64; 2]; 5] = [[1, 10], [2, 20], [1, 30], [3, 10], [1, 10]];

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_groups_greatest_end_holds_the_row_of_the_greatest_order"),
        word: Word::Succeeds,
        input: || Filled::EndsGreatest,
        expect: Expect::Value(Cow::Borrowed(
            "1 30; 2 20; 3 10 | orders [] | read 25 given 18 made 3 sides 0 0 | reached [Relation(RelationNumber(0))] | the language was asked 1 times",
        )),
    },
    Case {
        name: CaseName("a_groups_least_end_holds_every_row_tied_for_it"),
        word: Word::Succeeds,
        input: || Filled::EndsLeast,
        expect: Expect::Value(Cow::Borrowed(
            "1 10; 1 10; 2 20; 3 10 | orders [] | read 25 given 19 made 4 sides 0 0 | reached [Relation(RelationNumber(0))] | the language was asked 1 times",
        )),
    },
    Case {
        name: CaseName("an_ordered_match_pairs_first_in_first_out_the_left_row_then_the_right"),
        word: Word::Succeeds,
        input: || Filled::MatchOneKey,
        expect: Expect::Value(Cow::Borrowed(
            "7 1 10 7 2; 7 2 20 7 3 | orders [] | read 33 given 23 made 2 sides 3 3 | reached [Table(MadeTableId(0)), Table(MadeTableId(1))] | the language was asked 0 times",
        )),
    },
    Case {
        name: CaseName("an_ordered_match_pairs_within_each_key_alone"),
        word: Word::Succeeds,
        input: || Filled::MatchKeys,
        expect: Expect::Value(Cow::Borrowed(
            "8 2 20 8 3; 8 5 50 8 6 | orders [] | read 49 given 33 made 2 sides 4 5 | reached [Table(MadeTableId(0)), Table(MadeTableId(1))] | the language was asked 0 times",
        )),
    },
    Case {
        name: CaseName("a_key_that_holds_an_order_twice_is_the_meanings_fault"),
        word: Word::Refuses,
        input: || Filled::MatchDuplicate,
        expect: Expect::Refusal(|fault| {
            if matches!(fault, FilledError::Meaning { .. }) {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

/// The input of a group's end: the relation's two columns, keyed by the first and ordered by the second.
fn input() -> FillInput {
    let read = |column, out| FillStage::Read {
        source: StepRows::Filled(TOLD),
        column: ReadColumn::Filled(column),
        row: Register(0),
        out: Register(out),
    };
    FillInput {
        pipelines: Box::new([pipeline(
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
            [of(1), of(2)],
        )]),
        keys: ColumnList(Box::new([MadeColumn(0)])),
        order: Some(MadeColumn(1)),
    }
}

/// A constant table of the rows, and the pipeline that reads every column of it.
fn constant_input(number: u32, node: u8, rows: &[&[u64]]) -> (MadeTable, FillInput) {
    let width = rows.first().map_or(0, |row| row.len());
    let held = table(
        node,
        width,
        MadeFill::Constant(ConstantRows(
            rows.iter()
                .map(|row| row.iter().map(|cell| CellWord(*cell)).collect())
                .collect(),
        )),
    );
    let base = number.saturating_mul(10).saturating_add(20);
    let mut stages = vec![FillStage::Scan {
        source: StepRows::Made(MadeTableId(number)),
        columns: MatchColumns::default(),
        values: operands([]),
        out: Register(base),
    }];
    let mut row = Vec::new();
    for column in 0..u16::try_from(width).unwrap_or(0) {
        let out = base.saturating_add(1).saturating_add(u32::from(column));
        stages.push(FillStage::Read {
            source: StepRows::Made(MadeTableId(number)),
            column: ReadColumn::Made(MadeColumn(column)),
            row: Register(base),
            out: Register(out),
        });
        row.push(of(out));
    }
    (
        held,
        FillInput {
            pipelines: Box::new([pipeline_of(stages, row)]),
            keys: ColumnList(Box::new([MadeColumn(0)])),
            order: Some(MadeColumn(1)),
        },
    )
}

/// Two constant tables and their ordered match by the first column, ordered by the second.
fn matched(left: &[&[u64]], right: &[&[u64]]) -> Vec<MadeTable> {
    let (left_table, left_input) = constant_input(0, 2, left);
    let (right_table, right_input) = constant_input(1, 3, right);
    let width = left
        .first()
        .map_or(0, |row| row.len())
        .saturating_add(right.first().map_or(0, |row| row.len()));
    vec![
        left_table,
        right_table,
        table(
            1,
            width,
            MadeFill::Ordered(OrderedFill::Match(left_input, right_input)),
        ),
    ]
}

/// The tables of a case; the last is the one made.
fn tables(filled: Filled) -> Vec<MadeTable> {
    match filled {
        Filled::EndsGreatest | Filled::EndsLeast => vec![table(
            1,
            2,
            MadeFill::Group(
                GroupFill::Ends(if matches!(filled, Filled::EndsGreatest) {
                    GroupEnd::Greatest
                } else {
                    GroupEnd::Least
                }),
                input(),
            ),
        )],
        Filled::MatchOneKey => matched(
            &[&[7, 3, 30], &[7, 1, 10], &[7, 2, 20]],
            &[&[7, 3], &[7, 1], &[7, 2]],
        ),
        Filled::MatchKeys => matched(
            &[&[7, 1, 10], &[8, 5, 50], &[9, 1, 90], &[8, 2, 20]],
            &[&[8, 6], &[7, 1], &[8, 3], &[6, 9], &[8, 9]],
        ),
        Filled::MatchDuplicate => matched(&[&[7, 1, 10]], &[&[7, 2], &[7, 2]]),
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
        "{said} | orders {orders:?} | read {} given {} made {} sides {} {} | reached {reached:?} | the language was asked {} times",
        work.read.0,
        work.given.0,
        work.made.0,
        work.left.0,
        work.right.0,
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
