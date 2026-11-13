//! Partitioned closure outcomes through the mandatory public execution owner.
use super::super::super::fixtures;
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::Relation;
use ryo_relations::domain::relations::types::Value as Cell;
use ryo_values::cases::Case;
use ryo_values::cases::CaseName;
use ryo_values::cases::Expect;
use ryo_values::cases::Word;
use ryo_values::cases::run;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as Source;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::rows::errors::RowsError;
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::rows::types::Table;
use std::borrow::Cow;

/// One closure request: edges of `width` number columns, partitioned by the `keys` columns.
#[derive(Clone, Copy, Debug)]
struct Request {
    width: usize,
    edges: &'static [&'static [i64]],
    keys: &'static [u32],
}
/// What a partitioned closure is asked over: successive requests, each its own owner; edges reusing node ids across
/// two files, partitioned by file; or `n` partitions of the same three edges, interleaved.
#[derive(Clone, Copy, Debug)]
enum Partitions {
    Requests(&'static [Request]),
    ReusedNodeIds,
    Tiny(i64),
}
/// One request's answer: its schema, its row count, its number rows sorted then flattened, and any non-number cell.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Rows {
    kinds: Cow<'static, [CellKind]>,
    rows: usize,
    cells: Cow<'static, [i64]>,
    other_cells: usize,
}
#[derive(Debug, PartialEq, Eq)]
enum Observed {
    /// Each successive request's rows.
    Requests(Cow<'static, [Rows]>),
    /// The file-partitioned answer's schema and rows, and whether a pair crossed the files (node 1 reaching node 3).
    ReusedNodeIds {
        kinds: Cow<'static, [CellKind]>,
        rows: usize,
        crossed: bool,
    },
    /// The tiny partitions' rows and distinct rows.
    Tiny { rows: usize, distinct: usize },
}
type Input = Partitions;
type Value = Observed;
type Failure = OperatingError;
/// A table of `width` number columns, one row for each slice.
fn numbers(width: usize, rows: &[&[i64]]) -> Table {
    let mut table = Table {
        kinds: vec![CellKind::Number; width],
        ..Table::default()
    };
    for row in rows {
        push_row(
            &mut table,
            &row.iter()
                .map(|number| Cell::Number(Integer(*number)))
                .collect::<Vec<_>>(),
        );
    }
    table
}
/// The closure of `edges` partitioned by `keys`, through the completed owner: the caller never supplies workspace or a permit.
fn grouped(edges: &Table, keys: &[u32]) -> Result<Table, OperatingError> {
    fixtures::requests::table_request(
        Operation::Closure {
            edges: Source::Relation(Relation::Function),
            keys: keys.iter().copied().map(ColumnIndex).collect(),
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
        },
        &[edges],
        Vec::<ParameterBinding>::new(),
        &mut Symbols::default(),
    )
}
fn rows(out: Table) -> Rows {
    let width = out.kinds.len().max(1);
    let mut numbers: Vec<Vec<i64>> = out
        .cells
        .chunks_exact(width)
        .map(|row| {
            row.iter()
                .filter_map(|cell| {
                    if let Cell::Number(number) = cell {
                        Some(number.0)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .collect();
    numbers.sort_unstable();
    let cells: Vec<i64> = numbers.into_iter().flatten().collect();
    Rows {
        other_cells: out.cells.len() - cells.len(),
        rows: out.count.0,
        kinds: out.kinds.into(),
        cells: cells.into(),
    }
}
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Partitions::Requests(requests) => requests
            .iter()
            .map(|request| grouped(&numbers(request.width, request.edges), request.keys).map(rows))
            .collect::<Result<Vec<_>, _>>()
            .map(|answers| Observed::Requests(answers.into())),
        Partitions::ReusedNodeIds => {
            use ryo_relations::domain::relations::types::{FileId, NodeId};
            let mut edges = Table {
                kinds: vec![CellKind::Node, CellKind::Node, CellKind::File],
                ..Table::default()
            };
            for (from, to, file) in [(1, 2, 10), (2, 3, 20)] {
                push_row(
                    &mut edges,
                    &[
                        Cell::Node(NodeId(from)),
                        Cell::Node(NodeId(to)),
                        Cell::File(FileId(file)),
                    ],
                );
            }
            let out = grouped(&edges, &[2])?;
            Ok(Observed::ReusedNodeIds {
                crossed: out
                    .cells
                    .chunks_exact(3)
                    .any(|row| matches!(row, [_, Cell::Node(NodeId(1)), Cell::Node(NodeId(3))])),
                rows: out.count.0,
                kinds: out.kinds.into(),
            })
        }
        Partitions::Tiny(n) => {
            let mut edges = Table {
                kinds: vec![CellKind::Number; 3],
                ..Table::default()
            };
            // Interleave both partitions and repeated sources; every graph has just three edges.
            for edge in [[1, 2], [2, 3], [1, 4]] {
                for partition in 0..n {
                    push_row(
                        &mut edges,
                        &[
                            Cell::Number(Integer(edge[0])),
                            Cell::Number(Integer(edge[1])),
                            Cell::Number(Integer(partition)),
                        ],
                    );
                }
            }
            let out = grouped(&edges, &[2])?;
            let mut distinct: Vec<_> = out.cells.chunks_exact(3).collect();
            distinct.sort_unstable();
            distinct.dedup();
            Ok(Observed::Tiny {
                rows: out.count.0,
                distinct: distinct.len(),
            })
        }
    }
}
const fn request(width: usize, edges: &'static [&'static [i64]], keys: &'static [u32]) -> Request {
    Request { width, edges, keys }
}
const fn numbered(width: usize, rows: usize, cells: &'static [i64]) -> Rows {
    Rows {
        kinds: Cow::Borrowed(match width {
            2 => &[CellKind::Number; 2],
            3 => &[CellKind::Number; 3],
            _ => &[CellKind::Number; 4],
        }),
        rows,
        cells: Cow::Borrowed(cells),
        other_cells: 0,
    }
}
const fn answered(requests: &'static [Rows]) -> Expect<Value, Failure> {
    Expect::Value(Observed::Requests(Cow::Borrowed(requests)))
}
const INDEPENDENT: &[Rows] = &[numbered(3, 4, &[10, 1, 2, 10, 1, 4, 10, 2, 4, 20, 2, 3])];
const ORDERED: &[Rows] = &[numbered(
    4,
    5,
    &[
        100, 10, 1, 2, 100, 10, 1, 4, 100, 10, 2, 4, 100, 20, 1, 1, 200, 10, 2, 3,
    ],
)];
const CYCLES: &[Rows] = &[numbered(
    3,
    11,
    &[
        10, 1, 1, 10, 1, 2, 10, 1, 4, 10, 1, 5, 10, 2, 1, 10, 2, 2, 10, 2, 4, 10, 2, 5, 10, 4, 5,
        20, 1, 3, 20, 3, 3,
    ],
)];
const SUCCESSIVE: &[Rows] = &[
    numbered(3, 2, &[10, 1, 2, 20, 2, 3]),
    numbered(2, 3, &[1, 2, 1, 3, 2, 3]),
    numbered(3, 0, &[]),
    numbered(3, 1, &[30, 8, 9]),
    numbered(2, 3, &[1, 2, 1, 3, 2, 3]),
];
const NO_COLUMN: Expect<Value, Failure> =
    Expect::Error(OperatingError::Table(RowsError::NoSuchColumn { index: 1 }));
/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("file_partitions_keep_reused_node_ids_separate"),
        word: Word::Succeeds,
        input: || Partitions::ReusedNodeIds,
        expect: Expect::Value(Observed::ReusedNodeIds {
            kinds: Cow::Borrowed(&[CellKind::File, CellKind::Node, CellKind::Node]),
            rows: 2,
            crossed: false,
        }),
    },
    Case {
        name: CaseName("empty_partition_input_checks_endpoint_columns"),
        word: Word::Refuses,
        input: || Partitions::Requests(const { &[request(1, &[], &[0])] }),
        expect: NO_COLUMN,
    },
    Case {
        name: CaseName("partitions_are_independent"),
        word: Word::Succeeds,
        input: || {
            Partitions::Requests(
                const {
                    &[request(
                        3,
                        &[&[1, 2, 10], &[2, 3, 20], &[2, 4, 10], &[1, 2, 10]],
                        &[2],
                    )]
                },
            )
        },
        expect: answered(INDEPENDENT),
    },
    Case {
        name: CaseName("multiple_keys_keep_output_order"),
        word: Word::Succeeds,
        input: || {
            Partitions::Requests(
                const {
                    &[request(
                        4,
                        &[
                            &[1, 2, 10, 100],
                            &[2, 3, 10, 200],
                            &[2, 4, 10, 100],
                            &[1, 1, 20, 100],
                        ],
                        &[3, 2],
                    )]
                },
            )
        },
        expect: answered(ORDERED),
    },
    Case {
        name: CaseName("partitioned_cycles_and_starts"),
        word: Word::Succeeds,
        input: || {
            Partitions::Requests(
                const {
                    &[request(
                        3,
                        &[
                            &[1, 2, 10],
                            &[1, 3, 20],
                            &[2, 1, 10],
                            &[4, 5, 10],
                            &[1, 2, 10],
                            &[3, 3, 20],
                            &[1, 4, 10],
                        ],
                        &[2],
                    )]
                },
            )
        },
        expect: answered(CYCLES),
    },
    Case {
        name: CaseName("partitioned_empty_and_unpartitioned_owners_keep_exact_outputs"),
        word: Word::Succeeds,
        input: || {
            Partitions::Requests(
                const {
                    &[
                        request(3, &[&[1, 2, 10], &[2, 3, 20]], &[2]),
                        request(2, &[&[1, 2], &[2, 3]], &[]),
                        request(3, &[], &[2]),
                        request(3, &[&[8, 9, 30]], &[2]),
                        request(2, &[&[1, 2], &[2, 3]], &[]),
                    ]
                },
            )
        },
        expect: answered(SUCCESSIVE),
    },
    Case {
        name: CaseName("many_tiny_partitions_bound_work_and_output"),
        word: Word::Succeeds,
        input: || Partitions::Tiny(2048),
        expect: Expect::Value(Observed::Tiny {
            rows: 8192,
            distinct: 8192,
        }),
    },
    Case {
        name: CaseName("empty_partition_input_still_checks_columns"),
        word: Word::Refuses,
        input: || Partitions::Requests(const { &[request(2, &[], &[9])] }),
        expect: Expect::Error(OperatingError::Table(RowsError::NoSuchColumn { index: 9 })),
    },
];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
