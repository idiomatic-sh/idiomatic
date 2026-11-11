//! Ordered graph traversals that succeed keep exact order, multiplicity and work counts.
use super::fixtures::{
    cycles_preserve_every_edge_visit, deep_chain_and_work_are_bounded,
    entry_node_encodes_initial_empty_class_seen, many_interleaved_partitions_bound_expansions,
    numeric_order_and_diamond, parallel_distinct_orders_preserve_multiplicity,
    partitions_and_multiple_keys_are_exact, roots_are_independent_and_duplicates_are_deduplicated,
};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

/// The traversal a row runs.
type Input = fn() -> Result<usize, OperatingError>;
/// How many rows the traversal answered.
type Value = usize;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
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

/// Runs the row's traversal.
fn subject(input: Input) -> Result<Value, Failure> {
    super::super::fixtures::subject(input)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
