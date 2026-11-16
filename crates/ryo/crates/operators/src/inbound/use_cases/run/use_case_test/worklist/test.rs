//! The worklist through `plan` and `run` only: the order an entity leaving the queue follows its edges, the queue it keeps,
//! the state each pop reads, the row each followed edge writes, each partition and root run apart, and the lattice, order and
//! fault-order refusals.
use super::super::super::fixtures::worklists::{
    CALLS, CHAIN, Graph, first_faults, observed, passed, traced,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use std::borrow::Cow;

#[derive(Clone, Copy)]
enum Scenario {
    Fan,
    QueuedPerChange,
    RaisedBeforePop,
    CycleOfNoChange,
    ObservedAndLatched,
    PartitionsApart,
    RootOfTwoStates,
    UnchangedNotQueued,
    OtherBeforeState,
    PoppedTwiceInOneState,
    CyclicLattice,
    TwoLeastStates,
    TwoJoins,
    UnknownRootState,
    TwoJoinsNoRunJoins,
    OneOrderTwice,
    OneOrderInTwoStates,
    InputFaults,
}
#[derive(Debug, PartialEq)]
enum Observation {
    /// The rows answered, each `root:source>destination#order before=after`, a partition and `/` first when keyed.
    Trace(Cow<'static, str>),
    /// How many rows were answered.
    Rows(usize),
    /// Which input's fault was answered with all three faulty, then with the lattice and the roots.
    Faults(Cow<'static, str>),
}
type Input = Scenario;
type Value = Observation;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_pop_follows_its_edges_in_ascending_order"),
        word: Word::Succeeds,
        input: || Scenario::Fan,
        expect: Expect::Value(Observation::Trace(Cow::Borrowed(""))),
    },
    Case {
        name: CaseName("a_destination_is_queued_once_for_each_change"),
        word: Word::Succeeds,
        input: || Scenario::QueuedPerChange,
        expect: Expect::Value(Observation::Trace(Cow::Borrowed(""))),
    },
    Case {
        name: CaseName("a_pop_reads_the_state_held_when_it_leaves_the_queue"),
        word: Word::Succeeds,
        input: || Scenario::RaisedBeforePop,
        expect: Expect::Value(Observation::Trace(Cow::Borrowed(""))),
    },
    Case {
        name: CaseName("a_join_that_changes_nothing_queues_nothing"),
        word: Word::Succeeds,
        input: || Scenario::CycleOfNoChange,
        expect: Expect::Value(Observation::Rows(0)),
    },
    Case {
        name: CaseName("every_followed_edge_is_one_row_with_the_destinations_state_after_the_join"),
        word: Word::Succeeds,
        input: || Scenario::ObservedAndLatched,
        expect: Expect::Value(Observation::Trace(Cow::Borrowed(""))),
    },
    Case {
        name: CaseName("each_partition_and_root_runs_alone_from_the_least_state"),
        word: Word::Succeeds,
        input: || Scenario::PartitionsApart,
        expect: Expect::Value(Observation::Trace(Cow::Borrowed(""))),
    },
    Case {
        name: CaseName("a_root_given_two_states_starts_at_their_join"),
        word: Word::Succeeds,
        input: || Scenario::RootOfTwoStates,
        expect: Expect::Value(Observation::Trace(Cow::Borrowed(""))),
    },
    Case {
        name: CaseName("an_unchanged_destination_is_not_queued_again"),
        word: Word::Succeeds,
        input: || Scenario::UnchangedNotQueued,
        expect: Expect::Value(Observation::Trace(Cow::Borrowed(""))),
    },
    Case {
        name: CaseName("an_edge_is_followed_only_in_its_before_state"),
        word: Word::Succeeds,
        input: || Scenario::OtherBeforeState,
        expect: Expect::Value(Observation::Trace(Cow::Borrowed(""))),
    },
    Case {
        name: CaseName("rows_are_distinct_in_first_written_order"),
        word: Word::Succeeds,
        input: || Scenario::PoppedTwiceInOneState,
        expect: Expect::Value(Observation::Trace(Cow::Borrowed(""))),
    },
    Case {
        name: CaseName("lattice_rows_whose_order_has_a_cycle_are_refused"),
        word: Word::Refuses,
        input: || Scenario::CyclicLattice,
        expect: Expect::Error(PortableOperatorFault::NotALattice),
    },
    Case {
        name: CaseName("lattice_rows_with_two_least_states_are_refused"),
        word: Word::Refuses,
        input: || Scenario::TwoLeastStates,
        expect: Expect::Error(PortableOperatorFault::NotALattice),
    },
    Case {
        name: CaseName("lattice_rows_with_two_least_upper_bounds_are_refused"),
        word: Word::Refuses,
        input: || Scenario::TwoJoins,
        expect: Expect::Error(PortableOperatorFault::NotALattice),
    },
    Case {
        name: CaseName("a_root_state_the_lattice_does_not_hold_is_refused"),
        word: Word::Refuses,
        input: || Scenario::UnknownRootState,
        expect: Expect::Error(PortableOperatorFault::NotALattice),
    },
    Case {
        name: CaseName("two_least_upper_bounds_no_run_joins_are_refused"),
        word: Word::Refuses,
        input: || Scenario::TwoJoinsNoRunJoins,
        expect: Expect::Error(PortableOperatorFault::NotALattice),
    },
    Case {
        name: CaseName("two_edges_of_one_source_and_before_state_with_one_order_are_refused"),
        word: Word::Refuses,
        input: || Scenario::OneOrderTwice,
        expect: Expect::Error(PortableOperatorFault::DuplicateOrder),
    },
    Case {
        name: CaseName("one_order_in_two_before_states_is_accepted"),
        word: Word::Succeeds,
        input: || Scenario::OneOrderInTwoStates,
        expect: Expect::Value(Observation::Trace(Cow::Borrowed(""))),
    },
    Case {
        name: CaseName("the_edges_fault_comes_before_the_lattices_and_the_roots"),
        word: Word::Succeeds,
        input: || Scenario::InputFaults,
        expect: Expect::Value(Observation::Faults(Cow::Borrowed(""))),
    },
];

/// A graph over the chain lattice of `roots` and `edges`, unkeyed.
fn chain(
    roots: &[(i64, &'static str)],
    edges: &[(i64, i64, i64, &'static str, &'static str)],
) -> Graph {
    Graph {
        lattice: CHAIN.to_vec(),
        roots: roots
            .iter()
            .map(|(root, state)| (0, *root, *state))
            .collect(),
        edges: edges
            .iter()
            .map(|(source, destination, order, before, after)| {
                (0, *source, *destination, *order, *before, *after)
            })
            .collect(),
        keyed: false,
    }
}

/// The lattice `rows`, a root at `a` and no edge.
fn lattice_of(rows: &[(&'static str, &'static str)], state: &'static str) -> Graph {
    Graph {
        lattice: rows.to_vec(),
        roots: vec![(0, 1, state)],
        edges: Vec::new(),
        keyed: false,
    }
}

/// The graph of each scenario.
fn graph(scenario: Scenario) -> Graph {
    match scenario {
        // Root 1 fans out to 4, 2 and 3 at orders 2, 0 and 1; each of them records its pop.
        Scenario::Fan => chain(
            &[(1, "a")],
            &[
                (1, 4, 2, "a", "a"),
                (1, 2, 0, "a", "a"),
                (1, 3, 1, "a", "a"),
                (2, 2, 0, "a", "u"),
                (3, 3, 0, "a", "u"),
                (4, 4, 0, "a", "u"),
            ],
        ),
        // constructor-super's transfer: 2 and 4 call super(); 6 is the exit, recording each pop.
        Scenario::QueuedPerChange => {
            let mut edges = passed(1, &[2, 5], false);
            edges.extend(passed(2, &[6, 4], true));
            edges.extend(passed(4, &[5, 6], true));
            edges.extend(passed(5, &[6], false));
            edges.extend(observed(
                6,
                &["never", "once", "mixed", "multiple"],
                "unreached",
            ));
            Graph {
                lattice: CALLS.to_vec(),
                roots: vec![(0, 1, "never")],
                edges,
                keyed: false,
            }
        }
        // 2 is raised to a, then to b, before it leaves the queue.
        Scenario::RaisedBeforePop => chain(
            &[(1, "a")],
            &[
                (1, 2, 0, "a", "a"),
                (1, 2, 1, "a", "b"),
                (2, 2, 0, "a", "u"),
                (2, 2, 0, "b", "u"),
            ],
        ),
        // A cycle that passes every state on unchanged.
        Scenario::CycleOfNoChange => chain(
            &[(1, "a")],
            &[
                (1, 2, 0, "a", "a"),
                (2, 3, 0, "a", "a"),
                (3, 1, 0, "a", "a"),
            ],
        ),
        // 1 records its pop; 2, raised to a, reads 1's state through a latch edge.
        Scenario::ObservedAndLatched => chain(
            &[(1, "b")],
            &[
                (1, 1, 0, "b", "u"),
                (1, 2, 1, "b", "a"),
                (2, 1, 0, "a", "u"),
            ],
        ),
        // Partition 10 has roots 1 and 5, partition 11 root 1; each partition's edges are its own.
        Scenario::PartitionsApart => Graph {
            lattice: CHAIN.to_vec(),
            roots: vec![(10, 1, "a"), (10, 5, "a"), (11, 1, "a")],
            edges: vec![
                (10, 1, 2, 0, "a", "b"),
                (10, 5, 2, 0, "a", "a"),
                (11, 1, 3, 0, "a", "a"),
            ],
            keyed: true,
        },
        // Root 1 is given never and once; it records its pop in every state.
        Scenario::RootOfTwoStates => Graph {
            lattice: CALLS.to_vec(),
            roots: vec![(0, 1, "never"), (0, 1, "once")],
            edges: observed(
                1,
                &["unreached", "never", "once", "mixed", "multiple"],
                "unreached",
            ),
            keyed: false,
        },
        // R=1 to B=2 and C=3; B reads L=4 through a latch; C raises L to b, then passes B the state it holds.
        Scenario::UnchangedNotQueued => chain(
            &[(1, "a")],
            &[
                (1, 2, 0, "a", "a"),
                (1, 3, 1, "a", "a"),
                (2, 4, 0, "a", "u"),
                (3, 4, 0, "a", "b"),
                (3, 2, 1, "a", "a"),
            ],
        ),
        // 1 holds a: its edge of before-state b is not followed.
        Scenario::OtherBeforeState => {
            chain(&[(1, "a")], &[(1, 2, 0, "a", "a"), (1, 3, 1, "b", "b")])
        }
        // 4 is queued from 2 at a and from 3 at b, and leaves the queue twice holding b.
        Scenario::PoppedTwiceInOneState => chain(
            &[(1, "a")],
            &[
                (1, 2, 0, "a", "a"),
                (1, 3, 1, "a", "a"),
                (2, 4, 0, "a", "a"),
                (3, 4, 0, "a", "b"),
                (4, 4, 0, "b", "u"),
            ],
        ),
        Scenario::CyclicLattice => lattice_of(&[("u", "a"), ("a", "b"), ("b", "a")], "a"),
        Scenario::TwoLeastStates => lattice_of(&[("u", "a"), ("v", "a")], "a"),
        // a and b have two least upper bounds, c and d, and the run joins them.
        Scenario::TwoJoins => {
            let mut graph = lattice_of(
                &[
                    ("u", "a"),
                    ("u", "b"),
                    ("a", "c"),
                    ("b", "c"),
                    ("a", "d"),
                    ("b", "d"),
                ],
                "a",
            );
            graph.edges = vec![(0, 1, 2, 0, "a", "a"), (0, 1, 2, 1, "a", "b")];
            graph
        }
        Scenario::UnknownRootState => lattice_of(CHAIN, "z"),
        // The same lattice, but no run joins a and b: the root holds a and has no edge.
        Scenario::TwoJoinsNoRunJoins => lattice_of(
            &[
                ("u", "a"),
                ("u", "b"),
                ("a", "c"),
                ("b", "c"),
                ("a", "d"),
                ("b", "d"),
            ],
            "a",
        ),
        Scenario::OneOrderTwice => chain(&[(1, "a")], &[(1, 2, 0, "a", "a"), (1, 3, 0, "a", "b")]),
        Scenario::OneOrderInTwoStates => {
            chain(&[(1, "a")], &[(1, 2, 0, "a", "b"), (1, 3, 0, "b", "b")])
        }
        Scenario::InputFaults => Graph::default(),
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::InputFaults => Ok(Observation::Faults(first_faults().into())),
        Scenario::CycleOfNoChange => {
            let trace = traced(&graph(scenario))?;
            Ok(Observation::Rows(
                trace.split(' ').filter(|row| !row.is_empty()).count(),
            ))
        }
        other => traced(&graph(other)).map(|trace| Observation::Trace(trace.into())),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
