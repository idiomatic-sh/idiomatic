//! Which graph fills the contract states: the width `graph_width` answers for a fill of each kind as its variant of
//! `GraphFill` says it, and none for each way a fill is not one. The encoding writes no graph table of a width
//! `graph_width` does not answer.

use super::super::fixtures::{EDGES, RING, STATES, VISITS, graph_input, graph_pipeline};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::records::types::{EdgeColumns, VisitColumns, WorklistColumns};
use crate::domain::lowered::graphs::service::graph_width;
use crate::domain::lowered::types::{FillInput, GraphFill};
use crate::domain::steps::types::MadeColumn;
use std::borrow::Cow;

/// The fill a row asks the width of.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// A `follow` as the contract states it, and each way one is not.
    FollowStated,
    /// An `ordered-dfs` as the contract states it, and each way one is not.
    WalkStated,
    /// A `worklist` as the contract states it, and each way one is not.
    WorklistStated,
    /// A `cycles` as the contract states it, and each way one is not.
    CyclesStated,
}

type Input = Scenario;
type Value = Cow<'static, str>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "a_follow_is_stated_by_its_edges_alone_unordered_and_a_table_two_wider_than_its_keys",
        ),
        word: Word::Succeeds,
        input: || Scenario::FollowStated,
        expect: Expect::Value(Cow::Borrowed(
            "stated Some(3); no key Some(2); ordered edges None; each told column past its row [None, None]; a key past its row None; two inputs None; no input None; pipelines of two widths None; no pipeline None",
        )),
    },
    Case {
        name: CaseName(
            "an_ordered_walk_is_stated_by_its_roots_and_its_ordered_edges_keyed_alike_and_a_table_five_wider_than_its_keys",
        ),
        word: Word::Succeeds,
        input: || Scenario::WalkStated,
        expect: Expect::Value(Cow::Borrowed(
            "stated Some(6); no key Some(5); unordered edges None; ordered roots None; edges keyed on two columns None; each told column past its row [None, None, None, None]; the edges alone None; a third input None",
        )),
    },
    Case {
        name: CaseName(
            "a_worklist_is_stated_by_its_roots_its_ordered_edges_and_an_unkeyed_lattice_and_a_table_six_wider_than_its_keys",
        ),
        word: Word::Succeeds,
        input: || Scenario::WorklistStated,
        expect: Expect::Value(Cow::Borrowed(
            "stated Some(7); no key Some(6); unordered edges None; ordered roots None; an ordered lattice None; a keyed lattice None; edges with no key None; each told column past its row [None, None, None, None, None, None, None, None]; no lattice None",
        )),
    },
    Case {
        name: CaseName(
            "a_cycles_is_stated_by_its_edges_alone_unkeyed_and_unordered_and_a_table_of_two_columns",
        ),
        word: Word::Succeeds,
        input: || Scenario::CyclesStated,
        expect: Expect::Value(Cow::Borrowed(
            "stated Some(2); keyed edges None; ordered edges None; each told column past its row [None, None]; two inputs None",
        )),
    },
];

/// The width of a fill over inputs, as a number.
fn width(fill: GraphFill, inputs: &[FillInput]) -> Option<usize> {
    graph_width(fill, inputs).map(|width| width.0)
}

/// The width of each `follow`, the stated one first.
fn follow_stated() -> String {
    let fill = GraphFill::FixedPoints(EDGES);
    let mut uneven = graph_input(3, &[0], None);
    uneven.pipelines = Box::new([graph_pipeline(3), graph_pipeline(4)]);
    let mut empty = graph_input(3, &[0], None);
    empty.pipelines = Box::new([]);
    let far = MadeColumn(3);
    let past = [
        EdgeColumns {
            source: far,
            ..EDGES
        },
        EdgeColumns {
            destination: far,
            ..EDGES
        },
    ]
    .map(|columns| {
        width(
            GraphFill::FixedPoints(columns),
            &[graph_input(3, &[0], None)],
        )
    });
    format!(
        "stated {:?}; no key {:?}; ordered edges {:?}; each told column past its row {past:?}; a key past its row {:?}; \
         two inputs {:?}; no input {:?}; pipelines of two widths {:?}; no pipeline {:?}",
        width(fill, &[graph_input(3, &[0], None)]),
        width(fill, &[graph_input(3, &[], None)]),
        width(fill, &[graph_input(3, &[0], Some(1))]),
        width(fill, &[graph_input(3, &[3], None)]),
        width(
            fill,
            &[graph_input(3, &[0], None), graph_input(3, &[0], None)]
        ),
        width(fill, &[]),
        width(fill, &[uneven]),
        width(fill, &[empty]),
    )
}

/// The width of each `ordered-dfs`, the stated one first.
fn walk_stated() -> String {
    let fill = GraphFill::OrderedDfs(VISITS);
    let roots = || graph_input(2, &[0], None);
    let edges = || graph_input(5, &[0], Some(3));
    let far = MadeColumn(5);
    let past = [
        VisitColumns {
            root: far,
            ..VISITS
        },
        VisitColumns {
            source: far,
            ..VISITS
        },
        VisitColumns {
            destination: far,
            ..VISITS
        },
        VisitColumns {
            edge: far,
            ..VISITS
        },
    ]
    .map(|columns| width(GraphFill::OrderedDfs(columns), &[roots(), edges()]));
    format!(
        "stated {:?}; no key {:?}; unordered edges {:?}; ordered roots {:?}; edges keyed on two columns {:?}; each told \
         column past its row {past:?}; the edges alone {:?}; a third input {:?}",
        width(fill, &[roots(), edges()]),
        width(
            fill,
            &[graph_input(2, &[], None), graph_input(5, &[], Some(3))]
        ),
        width(fill, &[roots(), graph_input(5, &[0], None)]),
        width(fill, &[graph_input(2, &[0], Some(1)), edges()]),
        width(fill, &[roots(), graph_input(5, &[0, 1], Some(3))]),
        width(fill, &[edges()]),
        width(fill, &[roots(), edges(), edges()]),
    )
}

/// The width of each `worklist`, the stated one first.
fn worklist_stated() -> String {
    let fill = GraphFill::Worklist(STATES);
    let roots = || graph_input(3, &[0], None);
    let edges = || graph_input(6, &[0], Some(3));
    let lattice = || graph_input(2, &[], None);
    let far = MadeColumn(6);
    let past = [
        WorklistColumns {
            root: far,
            ..STATES
        },
        WorklistColumns {
            state: far,
            ..STATES
        },
        WorklistColumns {
            source: far,
            ..STATES
        },
        WorklistColumns {
            destination: far,
            ..STATES
        },
        WorklistColumns {
            before: far,
            ..STATES
        },
        WorklistColumns {
            after: far,
            ..STATES
        },
        WorklistColumns {
            lower: far,
            ..STATES
        },
        WorklistColumns {
            upper: far,
            ..STATES
        },
    ]
    .map(|columns| width(GraphFill::Worklist(columns), &[roots(), edges(), lattice()]));
    let unkeyed = [
        graph_input(3, &[], None),
        graph_input(6, &[], Some(3)),
        lattice(),
    ];
    format!(
        "stated {:?}; no key {:?}; unordered edges {:?}; ordered roots {:?}; an ordered lattice {:?}; a keyed lattice \
         {:?}; edges with no key {:?}; each told column past its row {past:?}; no lattice {:?}",
        width(fill, &[roots(), edges(), lattice()]),
        width(fill, &unkeyed),
        width(fill, &[roots(), graph_input(6, &[0], None), lattice()]),
        width(fill, &[graph_input(3, &[0], Some(1)), edges(), lattice()]),
        width(fill, &[roots(), edges(), graph_input(2, &[], Some(0))]),
        width(fill, &[roots(), edges(), graph_input(2, &[0], None)]),
        width(fill, &[roots(), graph_input(6, &[], Some(3)), lattice()]),
        width(fill, &[roots(), edges()]),
    )
}

/// The width of each `cycles`, the stated one first.
fn cycles_stated() -> String {
    let fill = GraphFill::Cycles(RING);
    let far = MadeColumn(2);
    let past = [
        EdgeColumns {
            source: far,
            ..RING
        },
        EdgeColumns {
            destination: far,
            ..RING
        },
    ]
    .map(|columns| width(GraphFill::Cycles(columns), &[graph_input(2, &[], None)]));
    format!(
        "stated {:?}; keyed edges {:?}; ordered edges {:?}; each told column past its row {past:?}; two inputs {:?}",
        width(fill, &[graph_input(2, &[], None)]),
        width(fill, &[graph_input(2, &[0], None)]),
        width(fill, &[graph_input(2, &[], Some(0))]),
        width(
            fill,
            &[graph_input(2, &[], None), graph_input(2, &[], None)]
        ),
    )
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(Cow::Owned(match scenario {
        Scenario::FollowStated => follow_stated(),
        Scenario::WalkStated => walk_stated(),
        Scenario::WorklistStated => worklist_stated(),
        Scenario::CyclesStated => cycles_stated(),
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
