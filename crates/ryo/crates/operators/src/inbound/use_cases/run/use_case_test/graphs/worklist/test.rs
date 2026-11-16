//! The worklist through `plan` and `run` only, over graphs with no cycle, which end whatever the queue does: each partition
//! and root run apart, a destination left unchanged not queued again, an edge followed only in its before-state, a source
//! popped twice in one state reading its destination each time, and the lattice, order and fault-order refusals. The rows over
//! graphs with a cycle are `cycles`'.
use super::super::super::super::fixtures::edges::{CHAIN, Graph, chain, first_faults, traced};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use std::borrow::Cow;

#[derive(Clone, Copy)]
enum Scenario {
    PartitionsApart,
    UnchangedNotQueued,
    OtherBeforeState,
    CyclicLattice,
    TwoLeastStates,
    TwoJoins,
    UnknownRootState,
    TwoJoinsNoRunJoins,
    OneOrderTwice,
    OneOrderInTwoStates,
    InputFaults,
    LatchReadTwice,
    EmptyLattice,
    EdgeOverEmptyLattice,
}
#[derive(Debug, PartialEq)]
enum Observation {
    /// The rows answered, each `root:source>destination#order before=after`, a partition and `/` first when keyed.
    Trace(Cow<'static, str>),
    /// Which input's fault was answered with all three faulty, then with the lattice and the roots.
    Faults(Cow<'static, str>),
}
type Input = Scenario;
type Value = Observation;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_partition_and_root_runs_alone_from_the_least_state"),
        word: Word::Succeeds,
        input: || Scenario::PartitionsApart,
        expect: Expect::Value(Observation::Trace(Cow::Borrowed(
            "10/1:1>2#0 a=b 10/5:5>2#0 a=a 11/1:1>3#0 a=a",
        ))),
    },
    Case {
        name: CaseName(
            "a_source_popped_twice_in_one_state_writes_a_row_for_each_destination_state",
        ),
        word: Word::Succeeds,
        input: || Scenario::LatchReadTwice,
        expect: Expect::Value(Observation::Trace(Cow::Borrowed(
            "1:1>2#0 a=a 1:1>3#1 a=a 1:1>2#2 a=b 1:2>4#0 b=u 1:3>4#0 a=b 1:2>4#0 b=b",
        ))),
    },
    Case {
        name: CaseName("an_unchanged_destination_is_not_queued_again"),
        word: Word::Succeeds,
        input: || Scenario::UnchangedNotQueued,
        expect: Expect::Value(Observation::Trace(Cow::Borrowed(
            "1:1>2#0 a=a 1:1>3#1 a=a 1:2>4#0 a=u 1:3>4#0 a=b 1:3>2#1 a=a",
        ))),
    },
    Case {
        name: CaseName("an_edge_is_followed_only_in_its_before_state"),
        word: Word::Succeeds,
        input: || Scenario::OtherBeforeState,
        expect: Expect::Value(Observation::Trace(Cow::Borrowed("1:1>2#0 a=a"))),
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
        expect: Expect::Value(Observation::Trace(Cow::Borrowed("1:1>2#0 a=b"))),
    },
    Case {
        name: CaseName("a_lattice_with_no_state_and_no_rows_answers_no_rows"),
        word: Word::Succeeds,
        input: || Scenario::EmptyLattice,
        expect: Expect::Value(Observation::Trace(Cow::Borrowed(""))),
    },
    Case {
        name: CaseName("an_edge_state_an_empty_lattice_does_not_hold_is_refused"),
        word: Word::Refuses,
        input: || Scenario::EdgeOverEmptyLattice,
        expect: Expect::Error(PortableOperatorFault::NotALattice),
    },
    Case {
        name: CaseName("the_edges_fault_comes_before_the_lattices_and_the_roots"),
        word: Word::Succeeds,
        input: || Scenario::InputFaults,
        expect: Expect::Value(Observation::Faults(Cow::Borrowed("edges lattice"))),
    },
];

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
        Scenario::CyclicLattice => lattice_of(&[("u", "a"), ("a", "b"), ("b", "a")], "a"),
        Scenario::TwoLeastStates => lattice_of(&[("u", "a"), ("v", "a")], "a"),
        // a and b have two least upper bounds, c and d, below the top t, and the run joins them.
        Scenario::TwoJoins => {
            let mut graph = lattice_of(
                &[
                    ("u", "a"),
                    ("u", "b"),
                    ("a", "c"),
                    ("b", "c"),
                    ("a", "d"),
                    ("b", "d"),
                    ("c", "t"),
                    ("d", "t"),
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
                ("c", "t"),
                ("d", "t"),
            ],
            "a",
        ),
        Scenario::OneOrderTwice => chain(&[(1, "a")], &[(1, 2, 0, "a", "a"), (1, 3, 0, "a", "b")]),
        Scenario::OneOrderInTwoStates => {
            chain(&[(1, "a")], &[(1, 2, 0, "a", "b"), (1, 3, 0, "b", "b")])
        }
        // 1 queues 2 at a, 3 at a, then 2 again at b; 2 leaves the queue at b twice, reading 4 before and after 3 raises it.
        Scenario::LatchReadTwice => chain(
            &[(1, "a")],
            &[
                (1, 2, 0, "a", "a"),
                (1, 3, 1, "a", "a"),
                (1, 2, 2, "a", "b"),
                (2, 4, 0, "b", "u"),
                (3, 4, 0, "a", "b"),
            ],
        ),
        Scenario::InputFaults | Scenario::EmptyLattice => Graph::default(),
        // No lattice row, no root, and one edge whose states the lattice cannot hold.
        Scenario::EdgeOverEmptyLattice => Graph {
            edges: vec![(0, 1, 2, 0, "a", "a")],
            ..Graph::default()
        },
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::InputFaults => Ok(Observation::Faults(first_faults().into())),
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
