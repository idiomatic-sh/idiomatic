//! Original scan/transport/key assertions through pure scanning or real compiled execution.
use super::fixtures::{CheckedSink, context, files, indexed_join, project_into};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::replay::service::scan;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::{FileId, Integer, Value as Cell};
use ryo_values::domain::rows::errors::RowsError;
use std::borrow::Cow;
/// What a row replays rows through.
#[derive(Clone, Copy, Debug)]
enum Replay {
    /// A pure scan of `files` file rows into a taker that refuses its first row or takes all.
    Scan { files: u32, refuse: bool },
    /// The real compiled projection of column `column` of two file rows into a checking sink.
    Project { column: u32 },
    /// The real indexed join of the hosted rows on `key`, keyed by column `column`.
    Join { key: i64, column: u32 },
}

/// The cells passed on, and (scans and projections only) how often the taker was called and how
/// many of those calls did not borrow the original source context.
#[derive(Debug, PartialEq)]
struct Replayed {
    cells: Cow<'static, [Cell]>,
    calls: Option<(usize, usize)>,
}

/// The refusal, how often the taker was called before it, and how many of those calls did not
/// borrow the original source context.
#[derive(Debug, PartialEq)]
struct Refused {
    error: OperatingError,
    calls: (usize, usize),
}

type Input = Replay;
type Value = Replayed;
type Failure = Refused;

const fn number(value: i64) -> Cell {
    Cell::Number(Integer(value))
}

const FILES_2: &[Cell] = &[Cell::File(FileId(0)), Cell::File(FileId(1))];
const FILES_3: &[Cell] = &[
    Cell::File(FileId(0)),
    Cell::File(FileId(1)),
    Cell::File(FileId(2)),
];

const fn ok(cells: &'static [Cell], calls: Option<(usize, usize)>) -> Replayed {
    Replayed {
        cells: Cow::Borrowed(cells),
        calls,
    }
}

const fn err(error: OperatingError, calls: (usize, usize)) -> Refused {
    Refused { error, calls }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_scan_passes_on_every_row_of_a_table"),
        word: Word::Succeeds,
        input: || Replay::Scan {
            files: 3,
            refuse: false,
        },
        expect: Expect::Value(ok(FILES_3, Some((3, 0)))),
    },
    Case {
        name: CaseName("a_scan_stops_at_the_row_the_taker_refuses"),
        word: Word::Refuses,
        input: || Replay::Scan {
            files: 3,
            refuse: true,
        },
        expect: Expect::Error(err(OperatingError::NotRun, (1, 0))),
    },
    Case {
        name: CaseName("the_engine_sink_reads_the_original_source_context"),
        word: Word::Succeeds,
        input: || Replay::Project { column: 0 },
        expect: Expect::Value(ok(FILES_2, Some((2, 0)))),
    },
    Case {
        name: CaseName("a_real_step_passes_on_the_rows_it_computes"),
        word: Word::Succeeds,
        input: || Replay::Project { column: 0 },
        expect: Expect::Value(ok(FILES_2, Some((2, 0)))),
    },
    Case {
        name: CaseName("rows_the_real_computation_refuses_are_not_passed_on"),
        word: Word::Refuses,
        input: || Replay::Project { column: 99 },
        expect: Expect::Error(err(
            OperatingError::Table(RowsError::NoSuchColumn { index: 99 }),
            (0, 0),
        )),
    },
    Case {
        name: CaseName("a_key_finds_the_rows_that_have_it_in_input_order"),
        word: Word::Succeeds,
        input: || Replay::Join { key: 1, column: 0 },
        expect: Expect::Value(ok(&[number(10), number(30)], None)),
    },
    Case {
        name: CaseName("a_key_no_row_has_finds_nothing"),
        word: Word::Succeeds,
        input: || Replay::Join { key: 9, column: 0 },
        expect: Expect::Value(ok(&[], None)),
    },
    Case {
        name: CaseName("a_join_row_without_its_key_column_is_refused"),
        word: Word::Refuses,
        input: || Replay::Join { key: 1, column: 3 },
        expect: Expect::Error(err(
            OperatingError::Table(RowsError::NoSuchColumn { index: 3 }),
            (0, 0),
        )),
    },
    Case {
        name: CaseName("computed_inputs_use_the_common_typed_index_without_a_native_tree_finding"),
        word: Word::Succeeds,
        input: || Replay::Join { key: 2, column: 0 },
        expect: Expect::Value(ok(&[number(20)], None)),
    },
];

/// Replays the rows and reports what was passed on.
fn subject(replay: Input) -> Result<Value, Failure> {
    let refused = |calls| move |error| Refused { error, calls };
    match replay {
        Replay::Scan {
            files: count,
            refuse,
        } => context(|context| {
            let mut sink = CheckedSink {
                calls: 0,
                foreign_sources: 0,
                refuse,
                rows: vec![],
            };
            let scanned = scan(
                &Rows::Computed(&files(count)),
                &mut vec![],
                context,
                &mut sink,
            );
            scanned.map_err(refused((sink.calls, sink.foreign_sources)))?;
            Ok(ok_owned(
                sink.rows,
                Some((sink.calls, sink.foreign_sources)),
            ))
        }),
        Replay::Project { column } => {
            let mut sink = CheckedSink {
                calls: 0,
                foreign_sources: 0,
                refuse: false,
                rows: vec![],
            };
            let projected = project_into(column, &mut sink);
            projected.map_err(refused((sink.calls, sink.foreign_sources)))?;
            Ok(ok_owned(
                sink.rows,
                Some((sink.calls, sink.foreign_sources)),
            ))
        }
        Replay::Join { key, column } => {
            let joined = indexed_join(key, column).map_err(refused((0, 0)))?;
            Ok(ok_owned(joined.cells, None))
        }
    }
}

fn ok_owned(cells: Vec<Cell>, calls: Option<(usize, usize)>) -> Replayed {
    Replayed {
        cells: cells.into(),
        calls,
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
