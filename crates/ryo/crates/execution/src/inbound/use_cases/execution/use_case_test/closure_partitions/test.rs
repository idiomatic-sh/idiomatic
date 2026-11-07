//! Partitioned closure outcomes through the mandatory public execution owner.
use super::super::super::fixtures;
use ryo_values::selects;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::Relation;
use ryo_relations::domain::relations::types::Value as Cell;
use ryo_language::domain::symbols::types::Symbols;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::fixtures::ExecutionObservation;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::cases::Case;
use ryo_values::cases::CaseName;
use ryo_values::cases::Expect;
use ryo_values::cases::Word;
use ryo_values::cases::run;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as Source;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::query_bindings::types::ParameterBinding;

type Value = usize;
type Failure = OperatingError;
/// A scenario: what a row computes with the item, answering how many rows came out.
type Input = fn() -> Result<Value, OperatingError>;
const FROM: Source = Source::Relation(Relation::Function);
fn column(index: u32) -> ColumnIndex {
    ColumnIndex(index)
}
/// A table of numbers, one row for each slice.
fn numbers(rows: &[&[i64]]) -> Table {
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
/// The caller receives the measured completed owner; it never supplies workspace or a permit.
fn compute_with_observation(
    operation: &Operation,
    inputs: &[&Table],
    bindings: &[ParameterBinding],
    observed: &mut ExecutionObservation,
) -> Result<Table, OperatingError> {
    let (table, actual) = fixtures::requests::table_request(
        operation.clone(),
        inputs,
        bindings.to_vec(),
        &mut Symbols::default(),
    )?;
    *observed = actual;
    Ok(table)
}
fn closure(edges: &Table, observation: &mut ExecutionObservation) -> Result<Table, OperatingError> {
    compute_with_observation(
        &Operation::Closure {
            edges: FROM,
            keys: Vec::new(),
            source: column(0),
            destination: column(1),
        },
        &[edges],
        &[],
        observation,
    )
}
fn assert_pairs(out: &Table, expected: &[[i64; 2]]) {
    let mut actual: Vec<_> = out
        .cells
        .chunks_exact(2)
        .map(|row| {
            [
                *row.first().expect("source column"),
                *row.get(1).expect("destination column"),
            ]
        })
        .collect();
    actual.sort_unstable();
    let mut expected: Vec<_> = expected
        .iter()
        .map(|pair| pair.map(|n| Cell::Number(Integer(n))))
        .collect();
    expected.sort_unstable();
    assert_eq!(out.kinds, [CellKind::Number; 2]);
    assert_eq!(out.count.0, expected.len());
    assert_eq!(actual, expected);
}
fn grouped(
    edges: &Table,
    keys: &[u32],
    observation: &mut ExecutionObservation,
) -> Result<Table, OperatingError> {
    compute_with_observation(
        &Operation::Closure {
            edges: FROM,
            keys: keys.iter().copied().map(column).collect(),
            source: column(0),
            destination: column(1),
        },
        &[edges],
        &[],
        observation,
    )
}
fn assert_rows(out: &Table, width: usize, expected: &[&[i64]]) {
    let mut actual: Vec<_> = out
        .cells
        .chunks_exact(width)
        .map(<[Cell]>::to_vec)
        .collect();
    let mut wanted: Vec<Vec<_>> = expected
        .iter()
        .map(|row| {
            row.iter()
                .map(|number| Cell::Number(Integer(*number)))
                .collect()
        })
        .collect();
    actual.sort_unstable();
    wanted.sort_unstable();
    assert_eq!(out.kinds, vec![CellKind::Number; width]);
    assert_eq!(out.count.0, wanted.len());
    assert_eq!(actual, wanted);
}
fn partitions_do_not_cross() -> Result<Value, OperatingError> {
    let out = grouped(
        &numbers(&[&[1, 2, 10], &[2, 3, 20], &[2, 4, 10], &[1, 2, 10]]),
        &[2],
        &mut ExecutionObservation::default(),
    )?;
    assert_rows(
        &out,
        3,
        &[&[10, 1, 2], &[10, 1, 4], &[10, 2, 4], &[20, 2, 3]],
    );
    Ok(out.count.0)
}
fn multiple_partition_keys_keep_argument_order() -> Result<Value, OperatingError> {
    let out = grouped(
        &numbers(&[
            &[1, 2, 10, 100],
            &[2, 3, 10, 200],
            &[2, 4, 10, 100],
            &[1, 1, 20, 100],
        ]),
        &[3, 2],
        &mut ExecutionObservation::default(),
    )?;
    assert_rows(
        &out,
        4,
        &[
            &[100, 10, 1, 2],
            &[100, 10, 1, 4],
            &[100, 10, 2, 4],
            &[200, 10, 2, 3],
            &[100, 20, 1, 1],
        ],
    );
    Ok(out.count.0)
}
fn partitioned_cycles_and_disconnected_starts() -> Result<Value, OperatingError> {
    let out = grouped(
        &numbers(&[
            &[1, 2, 10],
            &[1, 3, 20],
            &[2, 1, 10],
            &[4, 5, 10],
            &[1, 2, 10],
            &[3, 3, 20],
            &[1, 4, 10],
        ]),
        &[2],
        &mut ExecutionObservation::default(),
    )?;
    assert_rows(
        &out,
        3,
        &[
            &[10, 1, 1],
            &[10, 1, 2],
            &[10, 1, 4],
            &[10, 1, 5],
            &[10, 2, 1],
            &[10, 2, 2],
            &[10, 2, 4],
            &[10, 2, 5],
            &[10, 4, 5],
            &[20, 1, 3],
            &[20, 3, 3],
        ],
    );
    Ok(out.count.0)
}
fn partitioned_empty_and_unpartitioned_owners_keep_exact_outputs() -> Result<Value, OperatingError>
{
    let mut observation = ExecutionObservation::default();
    let partitioned = grouped(
        &numbers(&[&[1, 2, 10], &[2, 3, 20]]),
        &[2],
        &mut observation,
    )?;
    assert_rows(&partitioned, 3, &[&[10, 1, 2], &[20, 2, 3]]);
    let ordinary = closure(&numbers(&[&[1, 2], &[2, 3]]), &mut observation)?;
    assert_pairs(&ordinary, &[[1, 2], [1, 3], [2, 3]]);
    let empty = Table {
        kinds: vec![CellKind::Number; 3],
        ..Table::default()
    };
    let empty_output = grouped(&empty, &[2], &mut observation)?;
    assert_rows(&empty_output, 3, &[]);
    assert_eq!(observation.closure_partition_rows, 0);
    let later_partition = grouped(&numbers(&[&[8, 9, 30]]), &[2], &mut observation)?;
    assert_rows(&later_partition, 3, &[&[30, 8, 9]]);
    let unpartitioned = grouped(&numbers(&[&[1, 2], &[2, 3]]), &[], &mut observation)?;
    assert_pairs(&unpartitioned, &[[1, 2], [1, 3], [2, 3]]);
    Ok(unpartitioned.count.0)
}
fn file_partitions_keep_reused_node_ids_separate() -> Result<Value, OperatingError> {
    use ryo_relations::domain::relations::types::{FileId, NodeId};
    let mut edges = Table {
        kinds: vec![CellKind::Node, CellKind::Node, CellKind::File],
        ..Table::default()
    };
    push_row(
        &mut edges,
        &[
            Cell::Node(NodeId(1)),
            Cell::Node(NodeId(2)),
            Cell::File(FileId(10)),
        ],
    );
    push_row(
        &mut edges,
        &[
            Cell::Node(NodeId(2)),
            Cell::Node(NodeId(3)),
            Cell::File(FileId(20)),
        ],
    );
    let out = grouped(&edges, &[2], &mut ExecutionObservation::default())?;
    assert_eq!(out.kinds, [CellKind::File, CellKind::Node, CellKind::Node]);
    assert_eq!(out.count.0, 2);
    assert!(
        !out.cells
            .chunks_exact(3)
            .any(|row| matches!(row, [_, Cell::Node(NodeId(1)), Cell::Node(NodeId(3))]))
    );
    Ok(out.count.0)
}
fn endpoints_are_checked_on_empty_partition_input() -> Result<Value, OperatingError> {
    let empty = Table {
        kinds: vec![CellKind::Number],
        ..Table::default()
    };
    grouped(&empty, &[0], &mut ExecutionObservation::default()).map(|out| out.count.0)
}
fn partition_columns_are_checked_on_empty_input() -> Result<Value, OperatingError> {
    let empty = Table {
        kinds: vec![CellKind::Number; 2],
        ..Table::default()
    };
    grouped(&empty, &[9], &mut ExecutionObservation::default()).map(|out| out.count.0)
}
fn many_tiny_partitions_bound_working_graph_and_output() -> Result<Value, OperatingError> {
    let n = 2048;
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
    let mut observation = ExecutionObservation::default();
    let out = grouped(&edges, &[2], &mut observation)?;
    assert_eq!(
        out.count.0,
        4 * usize::try_from(n).expect("positive partition count")
    );
    assert!(
        observation.closure_partition_rows <= 3,
        "all simultaneously live adjacency tables must fit one three-row partition"
    );
    assert!(
        observation.closure_partition_capacity < 64,
        "aggregate live Value capacity (adjacency, keys, sources and work rows), not only the last partition"
    );
    assert!(observation.closure_visited <= 4);
    let mut rows: Vec<_> = out.cells.chunks_exact(3).collect();
    rows.sort_unstable();
    rows.dedup();
    assert_eq!(rows.len(), out.count.0);
    Ok(out.count.0)
}
/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("file_partitions_keep_reused_node_ids_separate"),
        word: Word::Succeeds,
        input: || file_partitions_keep_reused_node_ids_separate,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("empty_partition_input_checks_endpoint_columns"),
        word: Word::Refuses,
        input: || endpoints_are_checked_on_empty_partition_input,
        expect: Expect::Refusal(|error| selects!(matches!(error, OperatingError::Table(..)))),
    },
    Case {
        name: CaseName("partitions_are_independent"),
        word: Word::Succeeds,
        input: || partitions_do_not_cross,
        expect: Expect::Value(4),
    },
    Case {
        name: CaseName("multiple_keys_keep_output_order"),
        word: Word::Succeeds,
        input: || multiple_partition_keys_keep_argument_order,
        expect: Expect::Value(5),
    },
    Case {
        name: CaseName("partitioned_cycles_and_starts"),
        word: Word::Succeeds,
        input: || partitioned_cycles_and_disconnected_starts,
        expect: Expect::Value(11),
    },
    Case {
        name: CaseName("partitioned_empty_and_unpartitioned_owners_keep_exact_outputs"),
        word: Word::Succeeds,
        input: || partitioned_empty_and_unpartitioned_owners_keep_exact_outputs,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("many_tiny_partitions_bound_work_and_output"),
        word: Word::Succeeds,
        input: || many_tiny_partitions_bound_working_graph_and_output,
        expect: Expect::Value(8192),
    },
    Case {
        name: CaseName("empty_partition_input_still_checks_columns"),
        word: Word::Refuses,
        input: || partition_columns_are_checked_on_empty_input,
        expect: Expect::Refusal(|error| selects!(matches!(error, OperatingError::Table(..)))),
    },];
fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
