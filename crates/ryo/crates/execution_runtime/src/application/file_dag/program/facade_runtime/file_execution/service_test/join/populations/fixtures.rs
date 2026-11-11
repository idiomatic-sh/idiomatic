//! The support of the `populations` cases.
use super::super::cases::fixtures::{node, stretch, table_of};
use super::super::*;
use fixtures::execution::*;
use fixtures::provider::*;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, FileId, Integer, Relation, Value as Cell},
};
use ryo_relations::domain::relations::{
    service::to_bits,
    types::{ColumnsView, FactColumn},
};
use ryo_relations::domain::terms::types::Symbol;
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::rows::service::row_in;
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, ReachLookup, Side},
    plans::types::Operation,
};
use std::borrow::Cow;

pub(super) fn functions(count: usize, dense: bool) -> Table {
    let mut table = Table {
        kinds: kinds(Relation::Function).to_vec(),
        ..Table::default()
    };
    for place in 0..count {
        push_row(
            &mut table,
            &[
                node(if dense { 1 } else { (place % 8 + 1) as u32 }),
                Cell::Symbol(Symbol(1)),
                Cell::File(FileId((place % 2) as u32)),
                stretch(place as u32, place as u32 + 1),
            ],
        );
    }
    table
}

/// Joins 256 functions on (file, node) to four typed right rows, supplied as `left` under the
/// optional `limit`.
fn composite(stored: bool, optional_limit: Option<usize>) -> Observed {
    let left = functions(256, false);
    let columns = (0..left.kinds.len())
        .map(|column| {
            FactColumn(
                left.cells
                    .chunks_exact(4)
                    .map(|row| to_bits(row[column]).0)
                    .collect(),
            )
        })
        .collect::<Vec<_>>();
    let right = table_of(
        &[CellKind::Node, CellKind::File, CellKind::Number],
        &[
            &[node(3), Cell::File(FileId(0)), Cell::Number(Integer(30))],
            &[node(1), Cell::File(FileId(0)), Cell::Number(Integer(10))],
            &[node(3), Cell::File(FileId(0)), Cell::Number(Integer(31))],
            &[node(7), Cell::File(FileId(1)), Cell::Number(Integer(70))],
        ],
    );
    let op = operation(
        &[2, 0],
        &[1, 0],
        &[(Side::Left, 3), (Side::Right, 2), (Side::Left, 0)],
        vec![],
        &[],
    );
    let supplied = if stored {
        Rows::Stored(ColumnsView {
            kinds: kinds(Relation::Function),
            columns: &columns,
        })
    } else {
        Rows::Computed(&left)
    };
    {
        {
            let actual = invoke(
                &op,
                [supplied, Rows::Computed(&right)],
                &mut Symbols::default(),
                b"",
                false,
                real_port(),
                Faults {
                    optional_limit,
                    ..Faults::default()
                },
            )
            .unwrap();
            let cells = actual.result.unwrap().cells;
            let mut expected = vec![];
            for place in 0..256 {
                let at = place % 8 + 1;
                if at == 1 {
                    expected.extend([
                        stretch(place, place + 1),
                        Cell::Number(Integer(10)),
                        node(1),
                    ]);
                }
                if at == 3 {
                    for payload in [30, 31] {
                        expected.extend([
                            stretch(place, place + 1),
                            Cell::Number(Integer(payload)),
                            node(3),
                        ]);
                    }
                }
            }
            Observed::Bag {
                rows: cells.len() / 3,
                first: Cow::Owned(cells.iter().take(9).cloned().collect()),
                places_in_order: cells == expected,
                affected: Cow::Owned(actual.affected),
            }
        }
    }
}
/// Joins 4097 functions of one node to 16 numbered right rows, with no optional retention.
fn dense() -> Observed {
    let left = functions(4097, true);
    let right = table_of(
        &[CellKind::Node, CellKind::Number],
        &(0..16)
            .map(|at| vec![node(1), Cell::Number(Integer(at))])
            .collect::<Vec<_>>()
            .iter()
            .map(Vec::as_slice)
            .collect::<Vec<_>>(),
    );
    let op = operation(
        &[0],
        &[0],
        &[(Side::Left, 3), (Side::Right, 1)],
        vec![],
        &[],
    );
    let actual = invoke(
        &op,
        [Rows::Computed(&left), Rows::Computed(&right)],
        &mut Symbols::default(),
        b"",
        false,
        real_port(),
        Faults {
            optional_limit: Some(0),
            ..Faults::default()
        },
    )
    .unwrap();
    let table = actual.result.unwrap();
    let last = table.count.0 - 1;
    let row = |at| {
        Cow::Owned(
            row_in(&table, ryo_values::domain::rows::types::RowIndex(at))
                .unwrap()
                .to_vec(),
        )
    };
    Observed::Dense {
        rows: table.count.0,
        first: row(0),
        last: row(last),
        places_in_order: table.cells.chunks_exact(2).enumerate().all(|(place, row)| {
            row == [
                stretch((place / 16) as u32, (place / 16 + 1) as u32),
                Cell::Number(Integer((place % 16) as i64)),
            ]
        }),
    }
}

/// Joins 256 functions to one unmatched node under a partial entry population.
fn partial() -> Observed {
    let left = functions(256, false);
    let right = table_of(&[CellKind::Node], &[&[node(99)]]);
    let actual = invoke(
        &operation(&[0], &[0], &[(Side::Left, 3)], vec![], &[]),
        [Rows::Computed(&left), Rows::Computed(&right)],
        &mut Symbols::default(),
        b"",
        false,
        real_port(),
        Faults {
            partials: vec![ryo_execution_domain::domain::coverage::types::Partial {
                relation: Relation::Entry,
                cause: ryo_values::domain::stored_facts::types::PartialCause(
                    "incomplete fixture".into(),
                ),
                lacking: ryo_execution_domain::domain::coverage::types::Lacking::Any,
                at: ryo_execution_domain::domain::coverage::types::PartialPlace::Repository,
            }],
            optional_limit: Some(0),
            ..Faults::default()
        },
    )
    .unwrap();
    Observed::Bag {
        rows: actual.result.unwrap().count.0,
        first: Cow::Borrowed(&[]),
        places_in_order: true,
        affected: Cow::Owned(actual.affected),
    }
}
/// The reach Join of a primary to a cyclic right side, and what each run is given.
#[derive(Clone, Copy, Debug)]
pub(super) enum Reach {
    /// An empty primary.
    EmptyPrimary,
    /// The right source refuses as unread.
    RightRefused,
    /// The left source refuses as not run.
    LeftRefused,
    /// The sink refuses the first row.
    SinkRefused,
    /// Two equal primary rows reach the cycle 1 to 9 and 1 to 2 to 3.
    Reached,
}

/// The population a row joins.
#[derive(Clone, Copy, Debug)]
pub(super) enum Population {
    /// 256 functions joined on a composite typed key, stored or computed, under an optional
    /// retention limit.
    Composite {
        stored: bool,
        optional_limit: Option<usize>,
    },
    /// 4097 functions of one node, each joined to 16 numbers.
    Dense,
    /// An unmatched primary under a partial population.
    PartialEmpty,
    /// A reach Join.
    Reach(Reach),
}

/// What the Join answered.
#[derive(Debug, PartialEq)]
pub(super) enum Observed {
    /// The output rows, the first three rows' cells, whether every place's rows follow the
    /// first eight places', and the files the output is partial for.
    Bag {
        rows: usize,
        first: Cow<'static, [Cell]>,
        places_in_order: bool,
        affected: Cow<'static, [Option<FileId>]>,
    },
    /// The output rows, the first and last rows, and whether every row pairs its place with its
    /// number.
    Dense {
        rows: usize,
        first: Cow<'static, [Cell]>,
        last: Cow<'static, [Cell]>,
        places_in_order: bool,
    },
    /// The delivered rows, the result's rows, the builds and derived builds, whether the right
    /// source was admitted, and the traversal queue left.
    Reached {
        delivered: Cow<'static, [Cell]>,
        rows: usize,
        builds: u64,
        derived_builds: usize,
        right_admitted: bool,
        queue: usize,
    },
}

/// A refused Join: the error and the rows delivered before it.
#[derive(Debug, PartialEq)]
pub(super) struct Refusal {
    pub(super) error: OperatingError,
    pub(super) delivered: usize,
}

/// Joins the population.
pub(super) fn exercise(population: Population) -> Result<Observed, Refusal> {
    match population {
        Population::Composite {
            stored,
            optional_limit,
        } => Ok(composite(stored, optional_limit)),
        Population::Dense => Ok(dense()),
        Population::PartialEmpty => Ok(partial()),
        Population::Reach(reach) => reached(reach),
    }
}

/// Runs the reach Join with the run's faults and primary.
fn reached(run: Reach) -> Result<Observed, Refusal> {
    let n = |n| Cell::Number(Integer(n));
    let left = table_of(&[CellKind::Number], &[&[n(1)], &[n(1)]]);
    let empty = table_of(&[CellKind::Number], &[]);
    let right = table_of(
        &[CellKind::Number; 2],
        &[&[n(1), n(9)], &[n(1), n(2)], &[n(9), n(1)], &[n(2), n(3)]],
    );
    let mut op = operation(&[0], &[0], &[(Side::Right, 1)], vec![], &[]);
    if let Operation::Join { reach, .. } = &mut op {
        *reach = Some(ReachLookup {
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
            keys: vec![],
            columns: vec![ColumnIndex(0), ColumnIndex(1)],
        });
    }
    let (primary, faults) = match run {
        Reach::EmptyPrimary => (&empty, Faults::default()),
        Reach::RightRefused => (
            &left,
            Faults {
                right: Some(OperatingError::NotRead),
                ..Faults::default()
            },
        ),
        Reach::LeftRefused => (
            &left,
            Faults {
                left: Some(OperatingError::NotRun),
                ..Faults::default()
            },
        ),
        Reach::SinkRefused => (
            &left,
            Faults {
                sink_after: Some(0),
                ..Faults::default()
            },
        ),
        Reach::Reached => (&left, Faults::default()),
    };
    let actual = invoke(
        &op,
        [Rows::Computed(primary), Rows::Computed(&right)],
        &mut Symbols::default(),
        b"",
        true,
        real_port(),
        faults,
    )
    .unwrap();
    match actual.result {
        Ok(table) => Ok(Observed::Reached {
            delivered: Cow::Owned(actual.prefix.concat()),
            rows: table.count.0,
            builds: actual.counts.0,
            derived_builds: actual.derived_builds.len(),
            right_admitted: actual.attempts.contains(&SourceAttempt::Available(1)),
            queue: actual.work.traversal_queue_len,
        }),
        Err(error) => Err(Refusal {
            error,
            delivered: actual.prefix.len(),
        }),
    }
}
