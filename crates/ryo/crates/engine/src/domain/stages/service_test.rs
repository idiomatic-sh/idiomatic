//! The `stages` service's cases: each loop over a small batch, and a loop that gives several rows for one going on where a
//! full batch stopped it.

use super::service::{
    compact, gather, keep, keep_in, latest, probe, probe_keep, product, read, scan,
};
use super::types::{BatchKey, BatchRow, BatchValue, Batches, CellTest, FanRows, KeptWhen, KeyPart};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::IndexSite;
use crate::domain::indexes::reading::service::read as index_of;
use crate::domain::indexes::service::{begin, finish, key_column};
use crate::domain::indexes::types::{ColumnView, FileStamp, IndexArena};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::{Count, Integer};
use ryo_values::domain::plan_orders::types::OrderBound;
use std::borrow::Cow;

/// The loop a case runs.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// The handles of rows 2 to 4.
    Scan,
    /// The source's rows for each probe, two rows a batch: the row in and the row found of every pair, batch after batch.
    Probe,
    /// The probes whose key the source holds, or with `Absent` those it does not.
    ProbeKeep(KeptWhen),
    /// Two rows against a source of three, four pairs a batch.
    Product,
    /// The rows of each probe's group nearest its bound on a side.
    Latest(OrderBound),
    /// A column of the source read by handles, then gathered by rows in.
    ReadAndGather,
    /// The rows a comparison of two columns keeps, and a column with those rows moved up.
    Keep(CellTest),
    /// The rows whose cell is in a set, or with `Absent` is not.
    KeepIn(KeptWhen),
}

type Input = Asked;
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The source's key column.
const SOURCE: [u64; 5] = [5, 6, 5, 7, 5];
/// The probes.
const PROBES: [u64; 4] = [5, 8, 6, 5];
/// A second column of the batch.
const OTHER: [u64; 4] = [6, 8, 2, 4];

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_scan_gives_each_row_its_handle"),
        word: Word::Succeeds,
        input: || Asked::Scan,
        expect: Expect::Value(Cow::Borrowed(&[2, 3, 4])),
    },
    Case {
        name: CaseName("a_probe_goes_on_where_a_full_batch_stopped_it"),
        word: Word::Succeeds,
        input: || Asked::Probe,
        expect: Expect::Value(Cow::Borrowed(&[
            0, 0, 0, 2, 100, 0, 4, 2, 1, 100, 3, 0, 3, 2, 100, 3, 4, 100,
        ])),
    },
    Case {
        name: CaseName("a_probe_keeps_the_rows_whose_key_is_held"),
        word: Word::Succeeds,
        input: || Asked::ProbeKeep(KeptWhen::Present),
        expect: Expect::Value(Cow::Borrowed(&[0, 2, 3])),
    },
    Case {
        name: CaseName("an_absent_probe_keeps_the_rows_whose_key_is_not_held"),
        word: Word::Succeeds,
        input: || Asked::ProbeKeep(KeptWhen::Absent),
        expect: Expect::Value(Cow::Borrowed(&[1])),
    },
    Case {
        name: CaseName("a_product_gives_every_pair_once"),
        word: Word::Succeeds,
        input: || Asked::Product,
        expect: Expect::Value(Cow::Borrowed(&[
            0, 0, 0, 1, 0, 2, 1, 0, 100, 1, 1, 1, 2, 100,
        ])),
    },
    Case {
        name: CaseName("latest_before_gives_the_rows_tied_below_the_bound"),
        word: Word::Succeeds,
        input: || Asked::Latest(OrderBound::Before),
        expect: Expect::Value(Cow::Borrowed(&[0, 0, 0, 1, 2, 3])),
    },
    Case {
        name: CaseName("latest_at_or_after_gives_the_first_rows_from_the_bound"),
        word: Word::Succeeds,
        input: || Asked::Latest(OrderBound::AtOrAfter),
        expect: Expect::Value(Cow::Borrowed(&[0, 2, 3, 0, 3, 1])),
    },
    Case {
        name: CaseName("a_read_follows_handles_and_a_gather_follows_rows_in"),
        word: Word::Succeeds,
        input: || Asked::ReadAndGather,
        expect: Expect::Value(Cow::Borrowed(&[7, 5, 5, 100, 4, 6, 6])),
    },
    Case {
        name: CaseName("keep_equal"),
        word: Word::Succeeds,
        input: || Asked::Keep(CellTest::Equal),
        expect: Expect::Value(Cow::Borrowed(&[1, 100, 8])),
    },
    Case {
        name: CaseName("keep_less"),
        word: Word::Succeeds,
        input: || Asked::Keep(CellTest::Less),
        expect: Expect::Value(Cow::Borrowed(&[0, 100, 5])),
    },
    Case {
        name: CaseName("keep_greater_or_equal"),
        word: Word::Succeeds,
        input: || Asked::Keep(CellTest::GreaterOrEqual),
        expect: Expect::Value(Cow::Borrowed(&[1, 2, 3, 100, 8, 6, 5])),
    },
    Case {
        name: CaseName("keep_in_a_set"),
        word: Word::Succeeds,
        input: || Asked::KeepIn(KeptWhen::Present),
        expect: Expect::Value(Cow::Borrowed(&[0, 2, 3])),
    },
    Case {
        name: CaseName("keep_out_of_a_set"),
        word: Word::Succeeds,
        input: || Asked::KeepIn(KeptWhen::Absent),
        expect: Expect::Value(Cow::Borrowed(&[1])),
    },
];

/// A column of cells.
fn cells(numbers: &[u64]) -> Vec<CellWord> {
    numbers.iter().map(|number| CellWord(*number)).collect()
}

/// A column's cells as numbers.
fn numbers(column: &[CellWord]) -> Vec<u64> {
    column.iter().map(|cell| cell.0).collect()
}

/// A column that lies cell after cell.
fn plain(column: &[CellWord]) -> ColumnView<'_> {
    ColumnView {
        cells: column,
        first: Count(0),
        stride: Count(1),
    }
}

/// Runs a loop that gives several rows for one until it has read every row, and writes each batch as its pairs (row in,
/// cell out) and then 100.
fn batches(limit: usize, mut each: impl FnMut(&mut FanRows) -> Batches) -> Vec<u64> {
    let mut fan = FanRows {
        limit: Count(limit),
        ..FanRows::default()
    };
    let mut all = Vec::new();
    loop {
        fan.up.clear();
        fan.out.clear();
        let done = each(&mut fan);
        for (up, out) in fan.up.iter().zip(&fan.out) {
            all.extend([u64::from(up.0), out.0]);
        }
        all.push(100);
        if done == Batches::Done {
            break all;
        }
    }
}

fn subject(asked: Input) -> Result<Value, Failure> {
    let (source, probes, other) = (cells(&SOURCE), cells(&PROBES), cells(&OTHER));
    let mut arena = IndexArena::default();
    begin(&mut arena, Count(source.len()), Count(1));
    key_column(&mut arena, Count(0), plain(&source));
    finish(&mut arena, IndexSite(0), FileStamp(1), None);
    let index = index_of(&arena, IndexSite(0));
    let parts = [KeyPart {
        place: Count(0),
        column: &probes,
    }];
    let mut key = [CellWord(0)];
    let mut selection = Vec::new();
    let mut column = Vec::new();
    Ok(Cow::Owned(match asked {
        Asked::Scan => {
            scan(Count(2), Count(5), &mut column);
            numbers(&column)
        }
        Asked::Probe => batches(2, |fan| {
            let probed = BatchKey {
                cells: &mut key,
                parts: &parts,
            };
            probe(index, probed, Count(probes.len()), fan)
        }),
        Asked::ProbeKeep(kept) => {
            let probed = BatchKey {
                cells: &mut key,
                parts: &parts,
            };
            probe_keep(index, probed, Count(probes.len()), kept, &mut selection);
            selection.iter().map(|row| u64::from(row.0)).collect()
        }
        Asked::Product => batches(4, |fan| product(Count(2), Count(3), fan)),
        Asked::Latest(side) => {
            // A sorted table of two groups: key 5 at rows 0 to 2 with orders 1, 1, 4; key 6 at row 3 with order 2.
            let sorted = cells(&[5, 5, 5, 6]);
            let orders = [Integer(1), Integer(1), Integer(4), Integer(2)];
            let mut table = IndexArena::default();
            begin(&mut table, Count(sorted.len()), Count(1));
            key_column(&mut table, Count(0), plain(&sorted));
            finish(&mut table, IndexSite(0), FileStamp(1), None);
            // The probes 5, 8, 6, 5 with the bounds 3, 3, 9, 0.
            let bounds = cells(&[3, 3, 9, 0]);
            let mut all = batches(8, |fan| {
                let probed = BatchKey {
                    cells: &mut key,
                    parts: &parts,
                };
                latest(
                    index_of(&table, IndexSite(0)),
                    &orders,
                    probed,
                    BatchValue::Column(&bounds),
                    side,
                    Count(probes.len()),
                    fan,
                )
            });
            let _end: Option<u64> = all.pop();
            all
        }
        Asked::ReadAndGather => {
            let mut all = Vec::new();
            read(plain(&source), &cells(&[3, 0, 4]), &mut column);
            all.extend(numbers(&column));
            all.push(100);
            gather(
                &other,
                &[BatchRow(3), BatchRow(0), BatchRow(0)],
                &mut column,
            );
            all.extend(numbers(&column));
            all
        }
        Asked::Keep(test) => {
            keep(
                test,
                BatchValue::Column(&probes),
                BatchValue::Column(&other),
                Count(probes.len()),
                &mut selection,
            );
            let mut all: Vec<u64> = selection.iter().map(|row| u64::from(row.0)).collect();
            all.push(100);
            let mut moved = probes.clone();
            compact(&mut moved, &selection);
            all.extend(numbers(&moved));
            all
        }
        Asked::KeepIn(kept) => {
            keep_in(
                BatchValue::Column(&probes),
                &cells(&[5, 6]),
                kept,
                Count(probes.len()),
                &mut selection,
            );
            selection.iter().map(|row| u64::from(row.0)).collect()
        }
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
