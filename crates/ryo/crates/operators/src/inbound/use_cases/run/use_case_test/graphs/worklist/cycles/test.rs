//! The worklist through `plan` and `run` only, over graphs with a cycle (an edge back, or an entity's edge to itself): the
//! order an entity leaving the queue follows its edges, the queue it keeps, the state each pop reads and the row each followed
//! edge writes. A worklist that queued a destination on every join would never end these, so they are a test of their own,
//! apart from the rows that end whatever the queue does.
use super::super::super::super::super::fixtures::edges::{
    CALLS, Graph, chain, observed, passed, traced,
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
    RootOfTwoStates,
    PoppedTwiceInOneState,
}
#[derive(Debug, PartialEq)]
enum Observation {
    /// The rows answered, each `root:source>destination#order before=after`.
    Trace(Cow<'static, str>),
    /// How many rows were answered.
    Rows(usize),
}
type Input = Scenario;
type Value = Observation;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_pop_follows_its_edges_in_ascending_order"),
        word: Word::Succeeds,
        input: || Scenario::Fan,
        expect: Expect::Value(Observation::Trace(Cow::Borrowed(
            "1:1>2#0 a=a 1:1>3#1 a=a 1:1>4#2 a=a 1:2>2#0 a=a 1:3>3#0 a=a 1:4>4#0 a=a",
        ))),
    },
    Case {
        name: CaseName("a_destination_is_queued_once_for_each_change"),
        word: Word::Succeeds,
        input: || Scenario::QueuedPerChange,
        expect: Expect::Value(Observation::Trace(Cow::Borrowed(
            "1:1>2#0 never=never 1:1>5#1 never=never 1:2>6#0 never=once 1:2>4#1 never=once 1:5>6#0 never=mixed 1:6>6#0 mixed=mixed 1:4>5#0 once=multiple 1:4>6#1 once=multiple 1:6>6#0 multiple=multiple 1:5>6#0 multiple=multiple",
        ))),
    },
    Case {
        name: CaseName("a_pop_reads_the_state_held_when_it_leaves_the_queue"),
        word: Word::Succeeds,
        input: || Scenario::RaisedBeforePop,
        expect: Expect::Value(Observation::Trace(Cow::Borrowed(
            "1:1>2#0 a=a 1:1>2#1 a=b 1:2>2#0 b=b",
        ))),
    },
    Case {
        name: CaseName("a_join_that_changes_nothing_queues_nothing"),
        word: Word::Succeeds,
        input: || Scenario::CycleOfNoChange,
        expect: Expect::Value(Observation::Rows(3)),
    },
    Case {
        name: CaseName("every_followed_edge_is_one_row_with_the_destinations_state_after_the_join"),
        word: Word::Succeeds,
        input: || Scenario::ObservedAndLatched,
        expect: Expect::Value(Observation::Trace(Cow::Borrowed(
            "1:1>1#0 b=b 1:1>2#1 b=a 1:2>1#0 a=b",
        ))),
    },
    Case {
        name: CaseName("a_root_given_two_states_starts_at_their_join"),
        word: Word::Succeeds,
        input: || Scenario::RootOfTwoStates,
        expect: Expect::Value(Observation::Trace(Cow::Borrowed("1:1>1#0 mixed=mixed"))),
    },
    Case {
        name: CaseName("rows_are_distinct_in_first_written_order"),
        word: Word::Succeeds,
        input: || Scenario::PoppedTwiceInOneState,
        expect: Expect::Value(Observation::Trace(Cow::Borrowed(
            "1:1>2#0 a=a 1:1>3#1 a=a 1:2>4#0 a=a 1:3>4#0 a=b 1:4>4#0 b=b",
        ))),
    },
];

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
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let trace = traced(&graph(scenario))?;
    Ok(match scenario {
        Scenario::CycleOfNoChange => {
            Observation::Rows(trace.split(' ').filter(|cell| cell.contains('>')).count())
        }
        Scenario::Fan
        | Scenario::QueuedPerChange
        | Scenario::RaisedBeforePop
        | Scenario::ObservedAndLatched
        | Scenario::RootOfTwoStates
        | Scenario::PoppedTwiceInOneState => Observation::Trace(trace.into()),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
