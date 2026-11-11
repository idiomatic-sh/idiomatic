//! The support of the `succeeding` cases.
use super::super::super::super::*;
use super::super::fixtures::{compute, expected, numbers, operation};
use ryo_execution_domain::domain::operating::fixtures::ExecutionObservation;
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use ryo_values::cases::{Case, CaseName, Expect, Word};
use ryo_values::domain::rows::service::push_row;

pub(super) fn numeric_order_and_diamond() -> Result<usize, OperatingError> {
    let mut scratch = ExecutionObservation::default();
    let out = compute(
        &operation(&[], &[]),
        &numbers(1, &[&[1]]),
        &numbers(
            4,
            &[
                &[1, 2, 20, 12],
                &[2, 4, 0, 24],
                &[1, 3, -10, 13],
                &[3, 4, 0, 34],
            ],
        ),
        &mut scratch,
    )?;
    expected(
        &out,
        5,
        &[
            &[1, 1, 3, 13, 0],
            &[1, 3, 4, 34, 1],
            &[1, 1, 2, 12, 2],
            &[1, 2, 4, 24, 3],
        ],
    );
    assert_eq!(scratch.dfs_expanded, 4);
    assert_eq!(scratch.dfs_examined, 4);
    Ok(out.count.0)
}

pub(super) fn cycles_preserve_every_edge_visit() -> Result<usize, OperatingError> {
    let mut scratch = ExecutionObservation::default();
    let out = compute(
        &operation(&[], &[]),
        &numbers(1, &[&[1]]),
        &numbers(
            4,
            &[
                &[1, 1, 0, 11],
                &[1, 2, 1, 12],
                &[2, 1, 0, 21],
                &[2, 2, 1, 22],
            ],
        ),
        &mut scratch,
    )?;
    expected(
        &out,
        5,
        &[
            &[1, 1, 1, 11, 0],
            &[1, 1, 2, 12, 1],
            &[1, 2, 1, 21, 2],
            &[1, 2, 2, 22, 3],
        ],
    );
    assert_eq!(scratch.dfs_expanded, 2);
    assert_eq!(scratch.dfs_examined, 4);
    Ok(out.count.0)
}

pub(super) fn parallel_distinct_orders_preserve_multiplicity() -> Result<usize, OperatingError> {
    let out = compute(
        &operation(&[], &[]),
        &numbers(1, &[&[1]]),
        &numbers(4, &[&[1, 2, 1, 12], &[1, 2, 0, 12]]),
        &mut ExecutionObservation::default(),
    )?;
    expected(&out, 5, &[&[1, 1, 2, 12, 0], &[1, 1, 2, 12, 1]]);
    Ok(out.count.0)
}

pub(super) fn roots_are_independent_and_duplicates_are_deduplicated()
-> Result<usize, OperatingError> {
    let mut scratch = ExecutionObservation::default();
    let out = compute(
        &operation(&[], &[]),
        &numbers(1, &[&[2], &[1], &[1]]),
        &numbers(4, &[&[2, 3, 0, 23], &[1, 2, 0, 12]]),
        &mut scratch,
    )?;
    expected(
        &out,
        5,
        &[&[1, 1, 2, 12, 0], &[1, 2, 3, 23, 1], &[2, 2, 3, 23, 0]],
    );
    assert_eq!(scratch.dfs_expanded, 5);
    Ok(out.count.0)
}

pub(super) fn partitions_and_multiple_keys_are_exact() -> Result<usize, OperatingError> {
    let mut scratch = ExecutionObservation::default();
    let out = compute(
        &operation(&[2, 1], &[5, 4]),
        &numbers(
            3,
            &[&[1, 10, 100], &[1, 20, 100], &[1, 10, 200], &[1, 10, 100]],
        ),
        &numbers(
            6,
            &[
                &[1, 2, 0, 12, 10, 100],
                &[2, 3, 0, 23, 20, 100],
                &[1, 4, 0, 14, 20, 100],
                &[1, 5, 0, 15, 10, 200],
            ],
        ),
        &mut scratch,
    )?;
    expected(
        &out,
        7,
        &[
            &[100, 10, 1, 1, 2, 12, 0],
            &[100, 20, 1, 1, 4, 14, 0],
            &[200, 10, 1, 1, 5, 15, 0],
        ],
    );
    assert_eq!(scratch.dfs_examined, 3);
    Ok(out.count.0)
}

pub(super) fn entry_node_encodes_initial_empty_class_seen() -> Result<usize, OperatingError> {
    let out = compute(
        &operation(&[], &[]),
        &numbers(1, &[&[10]]),
        &numbers(
            4,
            &[
                &[10, 2, 0, 102],
                &[2, 1, 0, 21],
                &[1, 3, 0, 13],
                &[10, 3, 1, 103],
            ],
        ),
        &mut ExecutionObservation::default(),
    )?;
    expected(
        &out,
        5,
        &[
            &[10, 10, 2, 102, 0],
            &[10, 2, 1, 21, 1],
            &[10, 1, 3, 13, 2],
            &[10, 10, 3, 103, 3],
        ],
    );
    Ok(out.count.0)
}

pub(super) fn deep_chain_and_work_are_bounded() -> Result<usize, OperatingError> {
    let n = 20_000;
    let mut edges = numbers(4, &[]);
    for i in 0..n {
        push_row(
            &mut edges,
            &[
                Cell::Number(Integer(i)),
                Cell::Number(Integer(i + 1)),
                Cell::Number(Integer(0)),
                Cell::Number(Integer(i)),
            ],
        );
    }
    let mut scratch = ExecutionObservation::default();
    let out = compute(
        &operation(&[], &[]),
        &numbers(1, &[&[0]]),
        &edges,
        &mut scratch,
    )?;
    let expected_count = usize::try_from(n).expect("positive chain length");
    assert_eq!(out.count.0, expected_count);
    assert_eq!(scratch.dfs_examined, expected_count);
    assert_eq!(scratch.dfs_expanded, expected_count + 1);
    assert_eq!(scratch.dfs_frames_len, 0);
    Ok(out.count.0)
}

pub(super) fn many_interleaved_partitions_bound_expansions() -> Result<usize, OperatingError> {
    let n = 1024;
    let mut roots = numbers(2, &[]);
    let mut edges = numbers(5, &[]);
    for partition in 0..n {
        push_row(
            &mut roots,
            &[Cell::Number(Integer(1)), Cell::Number(Integer(partition))],
        );
    }
    for [source, dest, order, id] in [[1, 2, 0, 12], [2, 4, 0, 24], [1, 3, 1, 13], [3, 4, 0, 34]] {
        for partition in 0..n {
            push_row(
                &mut edges,
                &[
                    Cell::Number(Integer(source)),
                    Cell::Number(Integer(dest)),
                    Cell::Number(Integer(order)),
                    Cell::Number(Integer(id)),
                    Cell::Number(Integer(partition)),
                ],
            );
        }
    }
    let mut scratch = ExecutionObservation::default();
    let out = compute(&operation(&[1], &[4]), &roots, &edges, &mut scratch)?;
    let expected_count = 4 * usize::try_from(n).expect("positive partition count");
    assert_eq!(out.count.0, expected_count);
    assert_eq!(scratch.dfs_examined, out.count.0);
    assert_eq!(scratch.dfs_expanded, expected_count);
    assert!(scratch.dfs_visited_len <= 4);
    Ok(out.count.0)
}

pub(crate) type Input = fn() -> Result<usize, OperatingError>;

pub(crate) type Value = usize;

pub(crate) type Failure = OperatingError;

pub(crate) const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("numeric_depth_first_order_and_diamond"),
        word: Word::Succeeds,
        input: || numeric_order_and_diamond,
        expect: Expect::Value(4),
    },
    Case {
        name: CaseName("cycles_and_self_edges_are_emitted_before_seen_check"),
        word: Word::Succeeds,
        input: || cycles_preserve_every_edge_visit,
        expect: Expect::Value(4),
    },
    Case {
        name: CaseName("parallel_distinct_orders_preserve_visits"),
        word: Word::Succeeds,
        input: || parallel_distinct_orders_preserve_multiplicity,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("duplicate_roots_and_independent_walks"),
        word: Word::Succeeds,
        input: || roots_are_independent_and_duplicates_are_deduplicated,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("multiple_exact_partition_keys_keep_output_order"),
        word: Word::Succeeds,
        input: || partitions_and_multiple_keys_are_exact,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("caller_entry_node_encodes_initial_empty_seen"),
        word: Word::Succeeds,
        input: || entry_node_encodes_initial_empty_class_seen,
        expect: Expect::Value(4),
    },
    Case {
        name: CaseName("deep_chain_uses_iterative_frames"),
        word: Word::Succeeds,
        input: || deep_chain_and_work_are_bounded,
        expect: Expect::Value(20_000),
    },
    Case {
        name: CaseName("many_partitions_bound_work_without_timing"),
        word: Word::Succeeds,
        input: || many_interleaved_partitions_bound_expansions,
        expect: Expect::Value(4096),
    },
];
