//! File partitions keep identical node values separate and deliver rows in exact order.
use super::super::fixtures::{Ran, column, execute_request, host, hosted_plan, numbers, operation};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::fixtures::ExecutionObservation;
use ryo_execution_store::application::file_dag::derived::ledger::types::ByteCount;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, FileId, Integer, NodeId, Offset, Relation, Span, Value as Cell},
};
use ryo_values::domain::{
    plan_parts::types::Input as PlanInput,
    plans::types::Operation,
    rows::types::{RowCount, Table},
};
use std::borrow::Cow;

/// The walk a row runs.
#[derive(Clone, Copy, Debug)]
enum Walk {
    /// Semantic child edges `(parent, child, file)` walked from node 1 in each root file.
    FilePartitions {
        edges: &'static [[u32; 3]],
        roots: &'static [u32],
    },
    /// Hosted root and edge numbers, collected or delivered row by row.
    Hosted {
        roots: &'static [&'static [i64]],
        edges: &'static [&'static [i64]],
        delivery: bool,
    },
}

/// The table the walk answered.
#[derive(Debug, PartialEq)]
struct Walked {
    count: usize,
    kinds: Cow<'static, [CellKind]>,
    cells: Cow<'static, [Cell]>,
    /// The working bytes the graph owner still held after the file finished.
    retained: ByteCount,
}

type Input = Walk;
type Value = Walked;
type Failure = String;

const fn n(at: u64) -> Cell {
    Cell::Node(NodeId(at))
}

const fn number(value: i64) -> Cell {
    Cell::Number(Integer(value))
}

const HOSTED_ROOTS: &[&[i64]] = &[&[1], &[1]];
const HOSTED_EDGES: &[&[i64]] = &[&[1, 2, 1, 12], &[1, 3, 0, 13], &[3, 1, 0, 31]];

/// The hosted walk's rows in visit order, duplicates kept.
const HOSTED_CELLS: &[Cell] = &[
    number(1),
    number(1),
    number(3),
    number(13),
    number(0),
    number(1),
    number(3),
    number(1),
    number(31),
    number(1),
    number(1),
    number(1),
    number(2),
    number(12),
    number(2),
];

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("file_partition_keys_keep_identical_node_values_separate"),
        word: Word::Succeeds,
        input: || Walk::FilePartitions {
            edges: &[[1, 2, 10], [2, 3, 20], [1, 4, 20]],
            roots: &[10, 20],
        },
        expect: Expect::Value(Walked {
            count: 2,
            kinds: Cow::Borrowed(&[
                CellKind::File,
                CellKind::Node,
                CellKind::Node,
                CellKind::Node,
                CellKind::Node,
                CellKind::Number,
            ]),
            cells: Cow::Borrowed(&[
                Cell::File(FileId(10)),
                n(1),
                n(1),
                n(2),
                n(1),
                number(0),
                Cell::File(FileId(20)),
                n(1),
                n(1),
                n(4),
                n(1),
                number(0),
            ]),
            retained: ByteCount(0),
        }),
    },
    Case {
        name: CaseName("row_delivery_and_collected_visits_keep_exact_order_and_duplicates"),
        word: Word::Succeeds,
        input: || Walk::Hosted {
            roots: HOSTED_ROOTS,
            edges: HOSTED_EDGES,
            delivery: false,
        },
        expect: Expect::Value(Walked {
            count: 3,
            kinds: Cow::Borrowed(&[CellKind::Number; 5]),
            cells: Cow::Borrowed(HOSTED_CELLS),
            retained: ByteCount(0),
        }),
    },
    Case {
        name: CaseName("delivered_rows_are_the_collected_visits_in_the_same_order"),
        word: Word::Succeeds,
        input: || Walk::Hosted {
            roots: HOSTED_ROOTS,
            edges: HOSTED_EDGES,
            delivery: true,
        },
        expect: Expect::Value(Walked {
            count: 3,
            // The delivered rows are gathered from the sink, which carries no column kinds.
            kinds: Cow::Borrowed(&[]),
            cells: Cow::Borrowed(HOSTED_CELLS),
            retained: ByteCount(0),
        }),
    },
];

fn partitioned(edges: &[[u32; 3]], roots: &[u32]) -> Result<Ran, String> {
    let edge_cells = edges.iter().flat_map(|[parent, child, file]| {
        [
            n(u64::from(*parent)),
            n(u64::from(*child)),
            number(0),
            Cell::Symbol(ryo_relations::domain::terms::types::Symbol(0)),
            Cell::File(FileId(*file)),
        ]
    });
    let root_cells = roots.iter().flat_map(|file| {
        [
            n(1),
            Cell::File(FileId(*file)),
            Cell::Span(Span {
                start: Offset(0),
                end: Offset(0),
            }),
            number(1),
            number(1),
            number(0),
            number(0),
            number(0),
            number(0),
        ]
    });
    let providers = [
        (
            Relation::SemanticChild,
            Table {
                kinds: kinds(Relation::SemanticChild).to_vec(),
                cells: edge_cells.collect(),
                count: RowCount(edges.len()),
            },
        ),
        (
            Relation::SourceLocation,
            Table {
                kinds: kinds(Relation::SourceLocation).to_vec(),
                cells: root_cells.collect(),
                count: RowCount(roots.len()),
            },
        ),
    ];
    let mut request = hosted_plan(operation(&[1], &[4]), 0, 1);
    request.steps[0].operation = Operation::Select {
        from: PlanInput::Relation(Relation::SourceLocation),
        comparisons: vec![],
        kept: vec![column(0), column(1)],
    };
    // The edge identity is a real projected node column, rather than a synthetic provider schema.
    request.steps[1].operation = Operation::Select {
        from: PlanInput::Relation(Relation::SemanticChild),
        comparisons: vec![],
        kept: vec![column(0), column(1), column(2), column(0), column(4)],
    };
    execute_request(
        request,
        &[],
        &providers,
        &mut Symbols::default(),
        b"",
        false,
        &mut ExecutionObservation::default(),
    )
    .map_err(|error| format!("{error:?}"))
}

/// Runs the walk and answers its table.
fn subject(walk: Input) -> Result<Value, Failure> {
    let ran = match walk {
        Walk::FilePartitions { edges, roots } => partitioned(edges, roots)?,
        Walk::Hosted {
            roots,
            edges,
            delivery,
        } => execute_request(
            hosted_plan(operation(&[], &[]), 0, 1),
            &[
                host(&numbers(1, roots), "roots"),
                host(&numbers(4, edges), "edges"),
            ],
            &[],
            &mut Symbols::default(),
            b"",
            delivery,
            &mut ExecutionObservation::default(),
        )
        .map_err(|error| format!("{error:?}"))?,
    };
    let out = ran.answer.map_err(|error| format!("{error:?}"))?;
    Ok(Walked {
        count: out.count.0,
        kinds: out.kinds.into(),
        cells: out.cells.into(),
        retained: ran.retained,
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
