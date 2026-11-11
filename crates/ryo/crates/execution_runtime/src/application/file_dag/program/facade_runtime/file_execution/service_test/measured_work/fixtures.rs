//! The support of the `measured_work` cases.
use super::super::super::service::*;
use super::super::super::*;
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
use ryo_values::cases::{Case, CaseName, Expect, Word};
use ryo_values::domain::plan_parts::types::Direction;
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input as PlanInput, RepeatedRows},
    plans::types::Operation,
    rows::types::{RowCount, Table},
};

pub(crate) type Value = ();

pub(crate) type Failure = OperatingError;

pub(crate) type Input = fn() -> Result<Value, Failure>;

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

/// A later owner holds only its own partition rows, and an empty owner none.
pub(super) fn successive_owners_hold_only_their_own_partition_rows() -> Result<Value, Failure> {
    let (first, _) = closure(&numbers(&[&[1, 2], &[2, 3], &[1, 4]]), &[])?;
    assert_eq!(first.count.0, 4);
    let (later, observed) = closure(&numbers(&[&[5, 6]]), &[])?;
    assert_eq!(later.count.0, 1);
    assert_eq!(observed.closure_partition_rows, 1);
    let (cleared, observed) = closure(&empty(2), &[])?;
    assert_eq!(cleared.count.0, 0);
    assert_eq!(observed.closure_partition_rows, 0);
    let (_, observed) = closure(&empty(3), &[2])?;
    assert_eq!(observed.closure_partition_rows, 0);
    Ok(())
}

/// An interleaved closure visits each node of its walk at most once.
pub(super) fn interleaved_closure_visits_are_bounded() -> Result<Value, Failure> {
    let mut edges = empty(2);
    for i in 0..1024 {
        let middle = Cell::Number(Integer(2 * i + 1));
        push_row(&mut edges, &[Cell::Number(Integer(0)), middle]);
        push_row(&mut edges, &[middle, Cell::Number(Integer(2 * i + 2))]);
    }
    let (out, observed) = closure(&edges, &[])?;
    assert_eq!(out.count.0, 3072);
    assert!(observed.closure_visited > 0);
    assert!(observed.closure_visited <= 2049);
    Ok(())
}

/// Many tiny partitions keep one partition's adjacency, keys and work rows live at a time.
pub(super) fn many_tiny_partitions_bound_the_working_graph() -> Result<Value, Failure> {
    let n = 2048;
    let mut edges = empty(3);
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
    let (out, observed) = closure(&edges, &[2])?;
    assert_eq!(out.count.0, 4 * 2048);
    assert!(
        observed.closure_partition_rows <= 3,
        "all simultaneously live adjacency tables must fit one three-row partition"
    );
    assert!(
        observed.closure_partition_capacity < 64,
        "aggregate live Value capacity (adjacency, keys, sources and work rows), not only the last partition"
    );
    assert!(observed.closure_visited <= 4);
    // Positive reads: the bounds above cannot pass on counters that were never written.
    assert!(observed.closure_partition_rows > 0);
    assert!(observed.closure_partition_capacity > 0);
    assert!(observed.closure_visited > 0);
    Ok(())
}

pub(super) fn node(position: u32) -> Cell {
    Cell::Node(node_id(FileId(0), Position(position)))
}

/// A sparse computed-edge axis walk visits no unrelated component, including equal local positions in another file.
pub(super) fn sparse_axis_walk_visits_only_its_component() -> Result<Value, Failure> {
    let mut symbols = Symbols::default();
    for unrelated in [0_u32, 64, 4096] {
        for cycle in [false, true] {
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
            let (out, observed) = measured(axis, &[&starts, &edges, &domain], b"", &mut symbols)?;
            let targets = if cycle { 3 } else { 2 };
            assert_eq!(out.count.0, 2 * targets);
            assert_eq!(observed.closure_visited, if cycle { 4 } else { 3 });
        }
    }
    Ok(())
}

/// The former unary pilots hold the same exact row and other scratch when their input doubles.
pub(super) fn unary_pilots_keep_bounded_row_scratch() -> Result<Value, Failure> {
    // The exact row and other scratch each pilot holds: Select streams with none, so the positive reads of the other
    // three prove the counters are written.
    for (kind, scratch) in [
        (OperationKind::Select, (0, 0)),
        (OperationKind::Project, (0, 4)),
        (OperationKind::SourceMatches, (4, 0)),
        (OperationKind::SpanBounds, (4, 0)),
    ] {
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
        assert_eq!(second_rows.count.0, first_rows.count.0 * 2, "{kind:?}");
        assert_eq!(
            (first.row_capacity, first.other_capacity),
            scratch,
            "{kind:?}"
        );
        assert_eq!(
            (second.row_capacity, second.other_capacity),
            scratch,
            "{kind:?}"
        );
    }
    Ok(())
}

pub(crate) const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(stringify!(
            successive_owners_hold_only_their_own_partition_rows
        )),
        word: Word::Succeeds,
        input: || successive_owners_hold_only_their_own_partition_rows,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(interleaved_closure_visits_are_bounded)),
        word: Word::Succeeds,
        input: || interleaved_closure_visits_are_bounded,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(many_tiny_partitions_bound_the_working_graph)),
        word: Word::Succeeds,
        input: || many_tiny_partitions_bound_the_working_graph,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(sparse_axis_walk_visits_only_its_component)),
        word: Word::Succeeds,
        input: || sparse_axis_walk_visits_only_its_component,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(unary_pilots_keep_bounded_row_scratch)),
        word: Word::Succeeds,
        input: || unary_pilots_keep_bounded_row_scratch,
        expect: Expect::Value(()),
    },
];

pub(crate) fn subject(input: Input) -> Result<Value, Failure> {
    input()
}
