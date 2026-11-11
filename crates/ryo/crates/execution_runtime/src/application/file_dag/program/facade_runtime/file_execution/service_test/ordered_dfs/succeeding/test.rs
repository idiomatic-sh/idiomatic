//! Ordered graph traversals that succeed keep exact order, multiplicity and work counts.
use super::super::fixtures::{Answered, Listing, Observed, Work, n};
use super::fixtures::{
    cycles_preserve_every_edge_visit, deep_chain_and_work_are_bounded,
    entry_node_encodes_initial_empty_class_seen, many_interleaved_partitions_bound_expansions,
    numeric_order_and_diamond, parallel_distinct_orders_preserve_multiplicity,
    partitions_and_multiple_keys_are_exact, roots_are_independent_and_duplicates_are_deduplicated,
};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_store::application::file_dag::derived::ledger::types::ByteCount;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;

/// The traversal a row runs.
type Input = fn() -> Result<Observed, OperatingError>;
/// The rows the traversal answered, its work, and the bytes left after the file finished.
type Value = Observed;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("numeric_depth_first_order_and_diamond"),
        word: Word::Succeeds,
        input: || numeric_order_and_diamond,
        expect: Expect::Value(Observed {
            answered: Answered {
                kinds: Cow::Borrowed(&[
                    CellKind::Number,
                    CellKind::Number,
                    CellKind::Number,
                    CellKind::Number,
                    CellKind::Number,
                ]),
                count: 4,
                cells: 20,
                rows: Listing::Listed(Cow::Borrowed(&[
                    n(1),
                    n(1),
                    n(2),
                    n(12),
                    n(2),
                    n(1),
                    n(1),
                    n(3),
                    n(13),
                    n(0),
                    n(1),
                    n(2),
                    n(4),
                    n(24),
                    n(3),
                    n(1),
                    n(3),
                    n(4),
                    n(34),
                    n(1),
                ])),
            },
            work: Work {
                examined: 4,
                expanded: 4,
                frames: 0,
                visited: 4,
            },
            retained: ByteCount(0),
        }),
    },
    Case {
        name: CaseName("cycles_and_self_edges_are_emitted_before_seen_check"),
        word: Word::Succeeds,
        input: || cycles_preserve_every_edge_visit,
        expect: Expect::Value(Observed {
            answered: Answered {
                kinds: Cow::Borrowed(&[
                    CellKind::Number,
                    CellKind::Number,
                    CellKind::Number,
                    CellKind::Number,
                    CellKind::Number,
                ]),
                count: 4,
                cells: 20,
                rows: Listing::Listed(Cow::Borrowed(&[
                    n(1),
                    n(1),
                    n(1),
                    n(11),
                    n(0),
                    n(1),
                    n(1),
                    n(2),
                    n(12),
                    n(1),
                    n(1),
                    n(2),
                    n(1),
                    n(21),
                    n(2),
                    n(1),
                    n(2),
                    n(2),
                    n(22),
                    n(3),
                ])),
            },
            work: Work {
                examined: 4,
                expanded: 2,
                frames: 0,
                visited: 2,
            },
            retained: ByteCount(0),
        }),
    },
    Case {
        name: CaseName("parallel_distinct_orders_preserve_visits"),
        word: Word::Succeeds,
        input: || parallel_distinct_orders_preserve_multiplicity,
        expect: Expect::Value(Observed {
            answered: Answered {
                kinds: Cow::Borrowed(&[
                    CellKind::Number,
                    CellKind::Number,
                    CellKind::Number,
                    CellKind::Number,
                    CellKind::Number,
                ]),
                count: 2,
                cells: 10,
                rows: Listing::Listed(Cow::Borrowed(&[
                    n(1),
                    n(1),
                    n(2),
                    n(12),
                    n(0),
                    n(1),
                    n(1),
                    n(2),
                    n(12),
                    n(1),
                ])),
            },
            work: Work {
                examined: 2,
                expanded: 2,
                frames: 0,
                visited: 2,
            },
            retained: ByteCount(0),
        }),
    },
    Case {
        name: CaseName("duplicate_roots_and_independent_walks"),
        word: Word::Succeeds,
        input: || roots_are_independent_and_duplicates_are_deduplicated,
        expect: Expect::Value(Observed {
            answered: Answered {
                kinds: Cow::Borrowed(&[
                    CellKind::Number,
                    CellKind::Number,
                    CellKind::Number,
                    CellKind::Number,
                    CellKind::Number,
                ]),
                count: 3,
                cells: 15,
                rows: Listing::Listed(Cow::Borrowed(&[
                    n(1),
                    n(1),
                    n(2),
                    n(12),
                    n(0),
                    n(1),
                    n(2),
                    n(3),
                    n(23),
                    n(1),
                    n(2),
                    n(2),
                    n(3),
                    n(23),
                    n(0),
                ])),
            },
            work: Work {
                examined: 3,
                expanded: 5,
                frames: 0,
                visited: 3,
            },
            retained: ByteCount(0),
        }),
    },
    Case {
        name: CaseName("multiple_exact_partition_keys_keep_output_order"),
        word: Word::Succeeds,
        input: || partitions_and_multiple_keys_are_exact,
        expect: Expect::Value(Observed {
            answered: Answered {
                kinds: Cow::Borrowed(&[
                    CellKind::Number,
                    CellKind::Number,
                    CellKind::Number,
                    CellKind::Number,
                    CellKind::Number,
                    CellKind::Number,
                    CellKind::Number,
                ]),
                count: 3,
                cells: 21,
                rows: Listing::Listed(Cow::Borrowed(&[
                    n(100),
                    n(10),
                    n(1),
                    n(1),
                    n(2),
                    n(12),
                    n(0),
                    n(100),
                    n(20),
                    n(1),
                    n(1),
                    n(4),
                    n(14),
                    n(0),
                    n(200),
                    n(10),
                    n(1),
                    n(1),
                    n(5),
                    n(15),
                    n(0),
                ])),
            },
            work: Work {
                examined: 3,
                expanded: 6,
                frames: 0,
                visited: 2,
            },
            retained: ByteCount(0),
        }),
    },
    Case {
        name: CaseName("caller_entry_node_encodes_initial_empty_seen"),
        word: Word::Succeeds,
        input: || entry_node_encodes_initial_empty_class_seen,
        expect: Expect::Value(Observed {
            answered: Answered {
                kinds: Cow::Borrowed(&[
                    CellKind::Number,
                    CellKind::Number,
                    CellKind::Number,
                    CellKind::Number,
                    CellKind::Number,
                ]),
                count: 4,
                cells: 20,
                rows: Listing::Listed(Cow::Borrowed(&[
                    n(10),
                    n(1),
                    n(3),
                    n(13),
                    n(2),
                    n(10),
                    n(2),
                    n(1),
                    n(21),
                    n(1),
                    n(10),
                    n(10),
                    n(2),
                    n(102),
                    n(0),
                    n(10),
                    n(10),
                    n(3),
                    n(103),
                    n(3),
                ])),
            },
            work: Work {
                examined: 4,
                expanded: 4,
                frames: 0,
                visited: 4,
            },
            retained: ByteCount(0),
        }),
    },
    Case {
        name: CaseName("deep_chain_uses_iterative_frames"),
        word: Word::Succeeds,
        input: || deep_chain_and_work_are_bounded,
        expect: Expect::Value(Observed {
            answered: Answered {
                kinds: Cow::Borrowed(&[
                    CellKind::Number,
                    CellKind::Number,
                    CellKind::Number,
                    CellKind::Number,
                    CellKind::Number,
                ]),
                count: 20000,
                cells: 100000,
                rows: Listing::Ends {
                    first: Cow::Borrowed(&[n(0), n(0), n(1), n(0), n(0)]),
                    last: Cow::Borrowed(&[n(0), n(19999), n(20000), n(19999), n(19999)]),
                },
            },
            work: Work {
                examined: 20000,
                expanded: 20001,
                frames: 0,
                visited: 20001,
            },
            retained: ByteCount(0),
        }),
    },
    Case {
        name: CaseName("many_partitions_bound_work_without_timing"),
        word: Word::Succeeds,
        input: || many_interleaved_partitions_bound_expansions,
        expect: Expect::Value(Observed {
            answered: Answered {
                kinds: Cow::Borrowed(&[
                    CellKind::Number,
                    CellKind::Number,
                    CellKind::Number,
                    CellKind::Number,
                    CellKind::Number,
                    CellKind::Number,
                ]),
                count: 4096,
                cells: 24576,
                rows: Listing::Ends {
                    first: Cow::Borrowed(&[n(0), n(1), n(1), n(2), n(12), n(0)]),
                    last: Cow::Borrowed(&[n(1023), n(1), n(3), n(4), n(34), n(3)]),
                },
            },
            work: Work {
                examined: 4096,
                expanded: 4096,
                frames: 0,
                visited: 4,
            },
            retained: ByteCount(0),
        }),
    },
];

/// Runs the row's traversal.
fn subject(input: Input) -> Result<Value, Failure> {
    input()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
