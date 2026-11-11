//! The support of the `measured_work` cases.
use super::super::ordered_dfs::fixtures::execute_request;
use crate::application::file_dag::fixtures::operator_contract::expectations::SOURCE;
use crate::application::file_dag::fixtures::operator_contract::fixture;
use crate::application::file_dag::fixtures::operator_contract::inputs::table_inputs;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::fixtures::ExecutionObservation;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    service::node_id,
    types::{CellKind, FileId, Integer, Position, Relation, Value as Cell},
};
use ryo_values::domain::plan_parts::types::Direction;
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input as PlanInput, RepeatedRows},
    plans::types::Operation,
    rows::types::{RowCount, Table},
};

pub(super) const FROM: PlanInput = PlanInput::Relation(Relation::Function);

/// One operation over authored tables, answered with what its owner measured.
pub(super) fn measured(
    operation: Operation,
    inputs: &[&Table],
    source: &[u8],
    symbols: &mut Symbols,
) -> Result<(Table, ExecutionObservation), OperatingError> {
    let authored = table_inputs(operation, inputs, &[], symbols);
    let mut observed = ExecutionObservation::default();
    let table = execute_request(
        authored.plan,
        &authored.hosted,
        &authored.facts,
        symbols,
        source,
        false,
        &mut observed,
    )?;
    Ok((table, observed))
}

pub(super) fn numbers(rows: &[&[i64]]) -> Table {
    let width = rows.first().map_or(1, |row| row.len());
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

pub(super) fn closure(
    edges: &Table,
    keys: &[u32],
) -> Result<(Table, ExecutionObservation), OperatingError> {
    measured(
        Operation::Closure {
            edges: FROM,
            keys: keys.iter().copied().map(ColumnIndex).collect(),
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
        },
        &[edges],
        b"",
        &mut Symbols::default(),
    )
}

pub(super) fn empty(width: usize) -> Table {
    Table {
        kinds: vec![CellKind::Number; width],
        ..Table::default()
    }
}

/// Edges from 0 through each of `fans` middle nodes to its own end: an interleaved closure.
pub(super) fn interleaved(fans: i64) -> Table {
    let mut edges = empty(2);
    for i in 0..fans {
        let middle = Cell::Number(Integer(2 * i + 1));
        push_row(&mut edges, &[Cell::Number(Integer(0)), middle]);
        push_row(&mut edges, &[middle, Cell::Number(Integer(2 * i + 2))]);
    }
    edges
}

/// The edges 1-2, 2-3 and 1-4 in each of `partitions` partitions, keyed by the third column.
pub(super) fn partitioned(partitions: i64) -> Table {
    let mut edges = empty(3);
    for edge in [[1, 2], [2, 3], [1, 4]] {
        for partition in 0..partitions {
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
    edges
}

pub(super) fn node(position: u32) -> Cell {
    Cell::Node(node_id(FileId(0), Position(position)))
}

/// A computed-edge descendant axis from node 1 of file 0 (two starts) over the edges 1-4, 4-2
/// and 1-3, with 2-1 closing a cycle when `cycle`, beside a chain of `unrelated` edges in file 1
/// at equal local positions, within the domain 1, 2, 3: the rows it answered and the nodes it
/// visited.
pub(super) fn axis(unrelated: u32, cycle: bool) -> Result<(usize, usize), OperatingError> {
    let mut edges = Table {
        kinds: vec![CellKind::Node; 2],
        ..Table::default()
    };
    for [parent, child] in [[1, 4], [4, 2], [1, 3]] {
        push_row(&mut edges, &[node(parent), node(child)]);
    }
    if cycle {
        push_row(&mut edges, &[node(2), node(1)]);
    }
    for position in 1..=unrelated {
        push_row(
            &mut edges,
            &[
                Cell::Node(node_id(FileId(1), Position(position))),
                Cell::Node(node_id(FileId(1), Position(position + 1))),
            ],
        );
    }
    let mut starts = Table {
        kinds: vec![CellKind::Node, CellKind::Number],
        ..Table::default()
    };
    for payload in [11, 22] {
        push_row(&mut starts, &[node(1), Cell::Number(Integer(payload))]);
    }
    let mut domain = Table {
        kinds: vec![CellKind::Node],
        ..Table::default()
    };
    for position in [1, 2, 3] {
        push_row(&mut domain, &[node(position)]);
    }
    let axis = Operation::Axis {
        from: FROM,
        edges: FROM,
        domain: FROM,
        direction: Direction::Descendant,
        id: ColumnIndex(0),
        parent: ColumnIndex(0),
        child: ColumnIndex(1),
        domain_id: ColumnIndex(0),
        domain_filter: vec![],
        domain_view: Vec::new(),
        output: vec![],
    };
    let (out, observed) = measured(
        axis,
        &[&starts, &edges, &domain],
        b"",
        &mut Symbols::default(),
    )?;
    Ok((out.count.0, observed.closure_visited))
}

/// The rows a pilot answered over its input repeated 32 and 64 times, and the exact row and other
/// scratch each run held.
#[derive(Debug, PartialEq)]
pub(super) struct Pilot {
    pub(super) rows: (usize, usize),
    pub(super) small_scratch: (usize, usize),
    pub(super) large_scratch: (usize, usize),
}

/// The pilot of `kind` (a project keeping repeated rows) over its fixture's input repeated 32
/// and 64 times.
pub(super) fn pilot(kind: OperationKind) -> Result<Pilot, OperatingError> {
    let mut case = fixture(kind);
    if let Operation::Project { distinct, .. } = &mut case.operation {
        *distinct = RepeatedRows::Kept;
    }
    let repeated = |times: usize| Table {
        kinds: case.inputs[0].kinds.clone(),
        cells: case.inputs[0].cells.repeat(times),
        count: RowCount(case.inputs[0].count.0 * times),
    };
    let (small, large) = (repeated(32), repeated(64));
    let (first_rows, first) =
        measured(case.operation.clone(), &[&small], SOURCE, &mut case.symbols)?;
    let (second_rows, second) =
        measured(case.operation.clone(), &[&large], SOURCE, &mut case.symbols)?;
    Ok(Pilot {
        rows: (first_rows.count.0, second_rows.count.0),
        small_scratch: (first.row_capacity, first.other_capacity),
        large_scratch: (second.row_capacity, second.other_capacity),
    })
}
