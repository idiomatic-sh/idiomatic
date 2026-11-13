//! The closure work, walk visits and row scratch a real compiled owner measured, read from the runtime's own route
//! observation; the execution use case proves the same operations' answers.
use super::fixtures::{Pilot, axis, closure, empty, interleaved, numbers, partitioned, pilot};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_values::domain::query_operators::types::OperationKind;
use std::borrow::Cow;

/// The edges a closure walks.
#[derive(Clone, Copy, Debug)]
enum Edges {
    /// These number pairs.
    Rows(&'static [&'static [i64]]),
    /// No edges, in a table of this width.
    Empty(usize),
    /// Edges from 0 through each of this many middle nodes to its own end.
    Interleaved(i64),
    /// The edges 1-2, 2-3 and 1-4 in each of this many partitions, in the third column.
    Partitioned(i64),
}

/// One closure owner: its edges and the columns that key its partitions.
#[derive(Clone, Copy, Debug)]
struct Closure {
    edges: Edges,
    keys: &'static [u32],
}

/// The work a row measures.
#[derive(Clone, Copy, Debug)]
enum Workload {
    /// Closure owners, run one after another.
    Closures(&'static [Closure]),
    /// A descendant axis over a sparse computed-edge graph beside this many unrelated edges in
    /// another file, with a cycle or without.
    Axis { unrelated: u32, cycle: bool },
    /// The former unary pilot of this kind over a doubled input.
    Pilot(OperationKind),
}

/// What one closure owner answered and measured.
#[derive(Clone, Debug, PartialEq)]
struct ClosureWork {
    rows: usize,
    partition_rows: usize,
    partition_capacity: usize,
    visited: usize,
}

/// What the owner measured for the workload.
#[derive(Debug, PartialEq)]
enum Measured {
    /// Each closure owner's work, in order.
    Closures(Cow<'static, [ClosureWork]>),
    /// The axis's rows and the nodes its walk visited.
    Axis { rows: usize, visited: usize },
    /// The pilot's rows and scratch.
    Pilot(Pilot),
}

/// What the owner measured, and the working bytes each run left once its file finished, in run order.
#[derive(Debug, PartialEq)]
struct Answer {
    measured: Measured,
    retained: Cow<'static, [usize]>,
}

type Input = Workload;
type Value = Answer;
type Failure = OperatingError;

const fn work(
    rows: usize,
    partition_rows: usize,
    partition_capacity: usize,
    visited: usize,
) -> ClosureWork {
    ClosureWork {
        rows,
        partition_rows,
        partition_capacity,
        visited,
    }
}

const fn closures(work: &'static [ClosureWork]) -> Measured {
    Measured::Closures(Cow::Borrowed(work))
}

const fn unkeyed(edges: Edges) -> Closure {
    Closure { edges, keys: &[] }
}

/// The pilot's rows doubling from `rows` and its row and other scratch held the same.
const fn pilot_scratch(rows: usize, scratch: (usize, usize)) -> Measured {
    Measured::Pilot(Pilot {
        rows: (rows, 2 * rows),
        small_scratch: scratch,
        large_scratch: scratch,
    })
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("successive_owners_hold_only_their_own_partition_rows"),
        word: Word::Succeeds,
        input: || {
            Workload::Closures(
                const {
                    &[
                        unkeyed(Edges::Rows(&[&[1, 2], &[2, 3], &[1, 4]])),
                        unkeyed(Edges::Rows(&[&[5, 6]])),
                        unkeyed(Edges::Empty(2)),
                        Closure {
                            edges: Edges::Empty(3),
                            keys: &[2],
                        },
                    ]
                },
            )
        },
        expect: Expect::Value(Answer {
            measured: closures(&[
                work(4, 3, 31, 1),
                work(1, 1, 27, 1),
                work(0, 0, 0, 0),
                work(0, 0, 0, 0),
            ]),
            retained: Cow::Borrowed(&[0, 0, 0, 0]),
        }),
    },
    Case {
        name: CaseName("interleaved_closure_visits_are_bounded"),
        word: Word::Succeeds,
        input: || Workload::Closures(const { &[unkeyed(Edges::Interleaved(1024))] }),
        expect: Expect::Value(Answer {
            measured: closures(&[work(3072, 2048, 12808, 1)]),
            retained: Cow::Borrowed(&[0]),
        }),
    },
    Case {
        name: CaseName("many_tiny_partitions_bound_the_working_graph"),
        word: Word::Succeeds,
        input: || {
            Workload::Closures(
                const {
                    &[Closure {
                        edges: Edges::Partitioned(2048),
                        keys: &[2],
                    }]
                },
            )
        },
        expect: Expect::Value(Answer {
            measured: closures(&[work(4 * 2048, 3, 39, 1)]),
            retained: Cow::Borrowed(&[0]),
        }),
    },
    Case {
        name: CaseName("sparse_axis_walk_visits_only_its_component"),
        word: Word::Succeeds,
        input: || Workload::Axis {
            unrelated: 0,
            cycle: false,
        },
        expect: Expect::Value(Answer {
            measured: Measured::Axis {
                rows: 4,
                visited: 3,
            },
            retained: Cow::Borrowed(&[0]),
        }),
    },
    Case {
        name: CaseName("a_sparse_axis_walk_through_a_cycle_visits_only_its_component"),
        word: Word::Succeeds,
        input: || Workload::Axis {
            unrelated: 0,
            cycle: true,
        },
        expect: Expect::Value(Answer {
            measured: Measured::Axis {
                rows: 6,
                visited: 4,
            },
            retained: Cow::Borrowed(&[0]),
        }),
    },
    Case {
        name: CaseName("a_sparse_axis_walk_beside_64_unrelated_edges_visits_only_its_component"),
        word: Word::Succeeds,
        input: || Workload::Axis {
            unrelated: 64,
            cycle: false,
        },
        expect: Expect::Value(Answer {
            measured: Measured::Axis {
                rows: 4,
                visited: 3,
            },
            retained: Cow::Borrowed(&[0]),
        }),
    },
    Case {
        name: CaseName(
            "a_sparse_axis_walk_through_a_cycle_beside_64_unrelated_edges_visits_only_its_component",
        ),
        word: Word::Succeeds,
        input: || Workload::Axis {
            unrelated: 64,
            cycle: true,
        },
        expect: Expect::Value(Answer {
            measured: Measured::Axis {
                rows: 6,
                visited: 4,
            },
            retained: Cow::Borrowed(&[0]),
        }),
    },
    Case {
        name: CaseName("a_sparse_axis_walk_beside_4096_unrelated_edges_visits_only_its_component"),
        word: Word::Succeeds,
        input: || Workload::Axis {
            unrelated: 4096,
            cycle: false,
        },
        expect: Expect::Value(Answer {
            measured: Measured::Axis {
                rows: 4,
                visited: 3,
            },
            retained: Cow::Borrowed(&[0]),
        }),
    },
    Case {
        name: CaseName(
            "a_sparse_axis_walk_through_a_cycle_beside_4096_unrelated_edges_visits_only_its_component",
        ),
        word: Word::Succeeds,
        input: || Workload::Axis {
            unrelated: 4096,
            cycle: true,
        },
        expect: Expect::Value(Answer {
            measured: Measured::Axis {
                rows: 6,
                visited: 4,
            },
            retained: Cow::Borrowed(&[0]),
        }),
    },
    Case {
        name: CaseName("unary_pilots_keep_bounded_row_scratch"),
        word: Word::Succeeds,
        input: || Workload::Pilot(OperationKind::Select),
        expect: Expect::Value(Answer {
            measured: pilot_scratch(96, (0, 0)),
            retained: Cow::Borrowed(&[0, 0]),
        }),
    },
    Case {
        name: CaseName("the_project_pilot_keeps_bounded_row_scratch"),
        word: Word::Succeeds,
        input: || Workload::Pilot(OperationKind::Project),
        expect: Expect::Value(Answer {
            measured: pilot_scratch(96, (0, 4)),
            retained: Cow::Borrowed(&[0, 0]),
        }),
    },
    Case {
        name: CaseName("the_source_matches_pilot_keeps_bounded_row_scratch"),
        word: Word::Succeeds,
        input: || Workload::Pilot(OperationKind::SourceMatches),
        expect: Expect::Value(Answer {
            measured: pilot_scratch(64, (4, 0)),
            retained: Cow::Borrowed(&[0, 0]),
        }),
    },
    Case {
        name: CaseName("the_span_bounds_pilot_keeps_bounded_row_scratch"),
        word: Word::Succeeds,
        input: || Workload::Pilot(OperationKind::SpanBounds),
        expect: Expect::Value(Answer {
            measured: pilot_scratch(32, (4, 0)),
            retained: Cow::Borrowed(&[0, 0]),
        }),
    },
];

/// Runs the workload through its owner and reads what the owner measured.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Workload::Closures(owners) => {
            let mut measured = Vec::new();
            let mut retained = Vec::new();
            for owner in owners {
                let edges = match owner.edges {
                    Edges::Rows(rows) => numbers(rows),
                    Edges::Empty(width) => empty(width),
                    Edges::Interleaved(fans) => interleaved(fans),
                    Edges::Partitioned(partitions) => partitioned(partitions),
                };
                let (out, observed, left) = closure(&edges, owner.keys)?;
                retained.push(left);
                measured.push(work(
                    out.count.0,
                    observed.closure_partition_rows,
                    observed.closure_partition_capacity,
                    observed.closure_visited,
                ));
            }
            Ok(Answer {
                measured: Measured::Closures(measured.into()),
                retained: retained.into(),
            })
        }
        Workload::Axis { unrelated, cycle } => {
            let (rows, visited, left) = axis(unrelated, cycle)?;
            Ok(Answer {
                measured: Measured::Axis { rows, visited },
                retained: Cow::Owned(vec![left]),
            })
        }
        Workload::Pilot(kind) => {
            let (measured, retained) = pilot(kind)?;
            Ok(Answer {
                measured: Measured::Pilot(measured),
                retained: Cow::Owned(retained.to_vec()),
            })
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
