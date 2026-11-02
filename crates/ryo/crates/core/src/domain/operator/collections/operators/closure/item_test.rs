//! The `closure` item's cases.

use ryo_language::domain::relations::types::{CellKind, Relation, Value as Cell};
use ryo_values::domain::plans::types::QueryOperator;

use ryo_language::domain::symbols::types::Symbols;

use super::item::ClosureItem;
use crate::application::file_dag::testing::TestOperator as _;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::table_model::service::push_row;
use crate::domain::table_model::types::{ParameterBinding, Rows, Table};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::relations::types::Integer;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as Source;
use ryo_values::domain::plans::types::Operation;

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

fn compute(
    operation: &Operation,
    inputs: &[&Table],
    bindings: &[ParameterBinding],
) -> Result<Value, OperatingError> {
    compute_with_scratch(operation, inputs, bindings, &mut OperatorScratch::default())
        .map(|out| out.count.0)
}

fn compute_with_scratch(
    operation: &Operation,
    inputs: &[&Table],
    bindings: &[ParameterBinding],
    scratch: &mut OperatorScratch,
) -> Result<Table, OperatingError> {
    let mut symbols = Symbols::default();
    let rows: Vec<Rows<'_>> = inputs.iter().map(|table| Rows::Computed(table)).collect();
    let mut out = Table::default();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        stored_indexes: None,
        bindings,
        symbols: (&mut symbols).into(),
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch,
    };
    ClosureItem
        .compute(operation, &rows, &mut context, &mut out)
        .map(|()| out)
}

fn reaches_across_a_chain() -> Result<Value, OperatingError> {
    let operation = Operation::Closure {
        edges: FROM,
        keys: Vec::new(),
        source: column(0),
        destination: column(1),
    };
    compute(&operation, &[&numbers(&[&[1, 2], &[2, 3]])], &[])
}

fn closure(edges: &Table, scratch: &mut OperatorScratch) -> Result<Table, OperatingError> {
    compute_with_scratch(
        &Operation::Closure {
            edges: FROM,
            keys: Vec::new(),
            source: column(0),
            destination: column(1),
        },
        &[edges],
        &[],
        scratch,
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

fn interleaved_sources() -> Result<Value, OperatingError> {
    let out = closure(
        &numbers(&[&[1, 2], &[2, 3], &[1, 4]]),
        &mut OperatorScratch::default(),
    )?;
    assert_pairs(&out, &[[1, 2], [1, 3], [1, 4], [2, 3]]);
    Ok(out.count.0)
}

fn source_permutations() -> Result<Value, OperatingError> {
    let edges = [[1, 2], [2, 3], [1, 4]];
    let mut scratch = OperatorScratch::default();
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let rows: Vec<&[i64]> = order.iter().map(|i| edges[*i].as_slice()).collect();
        let out = closure(&numbers(&rows), &mut scratch)?;
        assert_pairs(&out, &[[1, 2], [1, 3], [1, 4], [2, 3]]);
    }
    Ok(6)
}

fn repeated_edges() -> Result<Value, OperatingError> {
    let mut scratch = OperatorScratch::default();
    for rows in [
        [&[1, 2][..], &[1, 2], &[2, 3], &[1, 4]],
        [&[1, 2][..], &[2, 3], &[1, 4], &[1, 2]],
    ] {
        let out = closure(&numbers(&rows), &mut scratch)?;
        assert_pairs(&out, &[[1, 2], [1, 3], [1, 4], [2, 3]]);
    }
    Ok(4)
}

fn cycles_and_self_edges() -> Result<Value, OperatingError> {
    let out = closure(
        &numbers(&[&[1, 2], &[2, 1], &[1, 2], &[1, 1]]),
        &mut OperatorScratch::default(),
    )?;
    assert_pairs(&out, &[[1, 1], [1, 2], [2, 1], [2, 2]]);
    Ok(out.count.0)
}

fn disconnected_sources() -> Result<Value, OperatingError> {
    let out = closure(
        &numbers(&[&[1, 2], &[3, 4], &[1, 5]]),
        &mut OperatorScratch::default(),
    )?;
    assert_pairs(&out, &[[1, 2], [1, 5], [3, 4]]);
    Ok(out.count.0)
}

fn scratch_is_reused_and_cleared() -> Result<Value, OperatingError> {
    let mut scratch = OperatorScratch::default();
    scratch.other.push(Cell::Number(Integer(999)));
    let initial = closure(&numbers(&[&[1, 2], &[2, 3], &[1, 4]]), &mut scratch)?;
    assert_pairs(&initial, &[[1, 2], [1, 3], [1, 4], [2, 3]]);
    assert!(scratch.observation.local_resources_released);
    let later = closure(&numbers(&[&[5, 6]]), &mut scratch)?;
    assert_pairs(&later, &[[5, 6]]);
    assert!(scratch.observation.local_resources_released);
    assert_eq!(scratch.observation.closure_partition_rows, 1);
    let empty = Table {
        kinds: vec![CellKind::Number; 2],
        ..Table::default()
    };
    let cleared = closure(&empty, &mut scratch)?;
    assert_pairs(&cleared, &[]);
    assert!(scratch.observation.local_resources_released);
    assert_eq!(scratch.observation.closure_partition_rows, 0);
    Ok(cleared.count.0)
}

fn interleaved_work_and_output_are_bounded() -> Result<Value, OperatingError> {
    let n = 1024;
    let mut edges = Table {
        kinds: vec![CellKind::Number; 2],
        ..Table::default()
    };
    for i in 0..n {
        let middle = Cell::Number(Integer(2 * i + 1));
        push_row(&mut edges, &[Cell::Number(Integer(0)), middle]);
        push_row(&mut edges, &[middle, Cell::Number(Integer(2 * i + 2))]);
    }
    let mut scratch = OperatorScratch::default();
    let out = closure(&edges, &mut scratch)?;
    let mut sources = out
        .cells
        .chunks_exact(2)
        .map(|row| row[0])
        .collect::<Vec<_>>();
    sources.sort_unstable();
    sources.dedup();
    assert_eq!(sources.len(), 1025);
    assert!(scratch.observation.closure_visited > 0);
    assert!(scratch.observation.closure_visited <= 2049);
    assert!(scratch.observation.local_resources_released);
    assert_eq!(out.count.0, 3072);
    assert_eq!(out.cells.len(), 6144);
    let mut pairs: Vec<_> = out.cells.chunks_exact(2).collect();
    pairs.sort_unstable();
    pairs.dedup();
    assert_eq!(pairs.len(), out.count.0);
    Ok(out.count.0)
}

fn closes_over_a_missing_column() -> Result<Value, OperatingError> {
    let operation = Operation::Closure {
        edges: FROM,
        keys: Vec::new(),
        source: column(0),
        destination: column(5),
    };
    compute(&operation, &[&numbers(&[&[1, 2]])], &[])
}

fn grouped(
    edges: &Table,
    keys: &[u32],
    scratch: &mut OperatorScratch,
) -> Result<Table, OperatingError> {
    compute_with_scratch(
        &Operation::Closure {
            edges: FROM,
            keys: keys.iter().copied().map(column).collect(),
            source: column(0),
            destination: column(1),
        },
        &[edges],
        &[],
        scratch,
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
        &mut OperatorScratch::default(),
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
        &mut OperatorScratch::default(),
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
        &mut OperatorScratch::default(),
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

fn partitioned_and_old_paths_reuse_scratch() -> Result<Value, OperatingError> {
    let mut scratch = OperatorScratch::default();
    let partitioned = grouped(&numbers(&[&[1, 2, 10], &[2, 3, 20]]), &[2], &mut scratch)?;
    assert_rows(&partitioned, 3, &[&[10, 1, 2], &[20, 2, 3]]);
    let ordinary = closure(&numbers(&[&[1, 2], &[2, 3]]), &mut scratch)?;
    assert_pairs(&ordinary, &[[1, 2], [1, 3], [2, 3]]);
    let empty = Table {
        kinds: vec![CellKind::Number; 3],
        ..Table::default()
    };
    let empty_output = grouped(&empty, &[2], &mut scratch)?;
    assert_rows(&empty_output, 3, &[]);
    assert!(scratch.observation.local_resources_released);
    assert_eq!(scratch.observation.closure_partition_rows, 0);
    let later_partition = grouped(&numbers(&[&[8, 9, 30]]), &[2], &mut scratch)?;
    assert_rows(&later_partition, 3, &[&[30, 8, 9]]);
    let unpartitioned = grouped(&numbers(&[&[1, 2], &[2, 3]]), &[], &mut scratch)?;
    assert_pairs(&unpartitioned, &[[1, 2], [1, 3], [2, 3]]);
    Ok(unpartitioned.count.0)
}

fn file_partitions_keep_reused_node_ids_separate() -> Result<Value, OperatingError> {
    use ryo_language::domain::relations::types::{FileId, NodeId};
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
    let out = grouped(&edges, &[2], &mut OperatorScratch::default())?;
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
    grouped(&empty, &[0], &mut OperatorScratch::default()).map(|out| out.count.0)
}

fn partition_columns_are_checked_on_empty_input() -> Result<Value, OperatingError> {
    let empty = Table {
        kinds: vec![CellKind::Number; 2],
        ..Table::default()
    };
    grouped(&empty, &[9], &mut OperatorScratch::default()).map(|out| out.count.0)
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
    let mut scratch = OperatorScratch::default();
    let out = grouped(&edges, &[2], &mut scratch)?;
    assert_eq!(
        out.count.0,
        4 * usize::try_from(n).expect("positive partition count")
    );
    assert!(
        scratch.observation.closure_partition_rows <= 3,
        "all simultaneously live adjacency tables must fit one three-row partition"
    );
    assert!(scratch.observation.local_resources_released);
    assert!(
        scratch.observation.closure_partition_capacity < 64,
        "aggregate live Value capacity (adjacency, keys, sources and work rows), not only the last partition"
    );
    assert!(scratch.observation.closure_visited <= 4);
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
        expect: Expect::Refusal(|error| selects(matches!(error, OperatingError::Table(..)))),
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
        name: CaseName("partitioned_old_empty_and_partitioned_reuse"),
        word: Word::Succeeds,
        input: || partitioned_and_old_paths_reuse_scratch,
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
        expect: Expect::Refusal(|error| selects(matches!(error, OperatingError::Table(..)))),
    },
    Case {
        name: CaseName("every_reachable_pair_is_listed"),
        word: Word::Succeeds,
        input: || reaches_across_a_chain,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("interleaved_sources_emit_each_pair_once"),
        word: Word::Succeeds,
        input: || interleaved_sources,
        expect: Expect::Value(4),
    },
    Case {
        name: CaseName("source_permutations_preserve_pairs"),
        word: Word::Succeeds,
        input: || source_permutations,
        expect: Expect::Value(6),
    },
    Case {
        name: CaseName("duplicate_edges_preserve_pairs"),
        word: Word::Succeeds,
        input: || repeated_edges,
        expect: Expect::Value(4),
    },
    Case {
        name: CaseName("cycles_include_only_reachable_reflexive_pairs"),
        word: Word::Succeeds,
        input: || cycles_and_self_edges,
        expect: Expect::Value(4),
    },
    Case {
        name: CaseName("disconnected_acyclic_sources_have_no_reflexive_pairs"),
        word: Word::Succeeds,
        input: || disconnected_sources,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("empty_input_clears_reused_scratch"),
        word: Word::Succeeds,
        input: || scratch_is_reused_and_cleared,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("interleaved_work_and_output_are_bounded"),
        word: Word::Succeeds,
        input: || interleaved_work_and_output_are_bounded,
        expect: Expect::Value(3072),
    },
    Case {
        name: CaseName("a_missing_column_is_refused"),
        word: Word::Refuses,
        input: || closes_over_a_missing_column,
        expect: Expect::Refusal(|error| selects(matches!(error, OperatingError::Table(..)))),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    super::super::fixtures::contract::check(QueryOperator::Closure);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
