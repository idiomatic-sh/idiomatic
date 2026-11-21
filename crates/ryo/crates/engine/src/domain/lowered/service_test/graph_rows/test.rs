//! The rows of the graph tables, each stated for a small input as a case of today's engine has it, every cell written at
//! the column `graph_column` gives it and the operation's own answer read back by place.
//!
//! `follow`: one key, 7, with the edges (1, 2), (2, 3), (3, 3) of the operators crate's case
//! `each_source_answers_its_fixed_point_and_a_self_pointer_answers_itself`, whose pairs are (1, 3), (2, 3), (3, 3).
//! `ordered-dfs`: the walk of root 1 of that crate's case `an_ordered_walk_follows_the_order_column` over the edges
//! (source, destination, order) (1, 3, 2), (1, 2, 1), (2, 3, 0), (3, 1, 0), each named by its order: its visits are
//! (1, 2, 1), (2, 3, 0), (3, 1, 0), (1, 3, 2), with the ordinals 0 to 3.
//! `worklist`: the fixture `operator_contract::worklist::fixture`, which the execution crate's case
//! `conformance_boundary` holds today's engine to: the lattice (0, 1), the root 1 in state 0, the edges (source,
//! destination, order, before, after) (1, 2, 0, 0, 1), (2, 2, 0, 1, 0); the edges followed are (1, 2, 0) from state 0 to
//! a destination in state 1, then (2, 2, 0) from state 1 to a destination in state 1.
//! `cycles`: the operators crate's case `cycles_are_marked_by_tarjan` over the edges (1, 2), (2, 1), (3, 3), (4, 5),
//! (5, 6), (9, 7), (7, 8), (8, 9), (9, 10), whose entities on a cycle are 1, 2, 3, 7, 8, 9.

use super::super::fixtures::{EDGES, RING, STATES, VISITS};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowered::graphs::service::{graph_column, graph_row};
use crate::domain::lowered::graphs::types::GraphCell;
use crate::domain::lowered::types::GraphFill;
use ryo_relations::domain::relations::types::Count;
use std::borrow::Cow;

/// What a row asks.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// The rows of a `follow` table for the small input, and the operation's answer read from them by place.
    FollowRows,
    /// The row of a `follow` of no key and of two, and the place of a cell its row does not have.
    FollowPlaces,
    /// The rows of an `ordered-dfs` table for the small input, and the operation's answer read from them by place.
    WalkRows,
    /// The rows of a `worklist` table for the small input, and the operation's answer read from them by place.
    WorklistRows,
    /// The rows of a `cycles` table for the small input, and the operation's answer read from them by place.
    CycleRows,
    /// The row of each fill with one key, and the place in each of a cell another fill's row has.
    EveryRow,
}

type Input = Scenario;
type Value = Cow<'static, str>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_follows_row_is_its_keys_the_source_and_the_fixed_point"),
        word: Word::Succeeds,
        input: || Scenario::FollowRows,
        expect: Expect::Value(Cow::Borrowed(
            "rows [[7, 1, 3], [7, 2, 3], [7, 3, 3]]; answer read by place [1, 3, 2, 3, 3, 3]",
        )),
    },
    Case {
        name: CaseName(
            "a_follows_keys_stand_first_in_their_order_and_a_cell_it_has_not_has_no_place",
        ),
        word: Word::Succeeds,
        input: || Scenario::FollowPlaces,
        expect: Expect::Value(Cow::Borrowed(
            "no key [Source, Destination]; two keys [Key(Count(0)), Key(Count(1)), Source, Destination]; the second key of two Some(MadeColumn(1)); a third key of two None",
        )),
    },
    Case {
        name: CaseName(
            "an_ordered_walks_row_is_its_keys_the_root_the_edge_and_the_ordinal_of_the_visit",
        ),
        word: Word::Succeeds,
        input: || Scenario::WalkRows,
        expect: Expect::Value(Cow::Borrowed(
            "rows [[7, 1, 1, 2, 1, 0], [7, 1, 2, 3, 0, 1], [7, 1, 3, 1, 0, 2], [7, 1, 1, 3, 2, 3]]; answer read by place [1, 0, 0, 1, 0, 2, 2, 3]",
        )),
    },
    Case {
        name: CaseName("a_worklists_row_is_its_keys_the_root_the_edge_followed_and_the_two_states"),
        word: Word::Succeeds,
        input: || Scenario::WorklistRows,
        expect: Expect::Value(Cow::Borrowed(
            "rows [[1, 1, 2, 0, 0, 1], [1, 2, 2, 0, 1, 1]]; answer read by place [0, 1, 1, 1]",
        )),
    },
    Case {
        name: CaseName("a_cycles_row_is_the_entity_twice"),
        word: Word::Succeeds,
        input: || Scenario::CycleRows,
        expect: Expect::Value(Cow::Borrowed(
            "rows [[1, 1], [2, 2], [3, 3], [7, 7], [8, 8], [9, 9]]; answer read by place [1, 1, 2, 2, 3, 3, 7, 7, 8, 8, 9, 9]",
        )),
    },
    Case {
        name: CaseName("each_fills_row_has_its_own_cells_and_no_place_for_anothers"),
        word: Word::Succeeds,
        input: || Scenario::EveryRow,
        expect: Expect::Value(Cow::Borrowed(
            "[Key(Count(0)), Source, Destination], the ordinal None, the state reached None; [Key(Count(0)), Root, Source, Destination, Edge, Ordinal], the ordinal Some(5), the state reached None; [Key(Count(0)), Root, Source, Destination, Order, Before, Reached], the ordinal None, the state reached Some(6); [Source, Destination], the ordinal None, the state reached None",
        )),
    },
];

/// A table's row: each of `cells` written at the column `graph_column` gives it, in a row as wide as `graph_row`.
fn row_of(fill: GraphFill, keys: Count, cells: &[(GraphCell, u64)]) -> Vec<u64> {
    let mut row = vec![u64::MAX; graph_row(fill, keys).len()];
    for (cell, value) in cells {
        let at = graph_column(fill, keys, *cell).map(|made| usize::from(made.0));
        if let Some(place) = at.and_then(|made| row.get_mut(made)) {
            *place = *value;
        }
    }
    row
}

/// The cells of `rows` at the columns of `asked`, row by row.
fn read(fill: GraphFill, keys: Count, rows: &[Vec<u64>], asked: &[GraphCell]) -> Vec<u64> {
    rows.iter()
        .flat_map(|row| {
            asked.iter().filter_map(|cell| {
                let at = graph_column(fill, keys, *cell)?;
                row.get(usize::from(at.0)).copied()
            })
        })
        .collect()
}

/// The rows of the small input's three pairs, and the operation's answer (the source, the fixed point) read by place.
fn follow_rows() -> String {
    let (fill, keys) = (GraphFill::FixedPoints(EDGES), Count(1));
    let pairs = [(1, 3), (2, 3), (3, 3)];
    let rows: Vec<Vec<u64>> = pairs
        .iter()
        .map(|(source, fixed)| {
            let cells = [
                (GraphCell::Key(Count(0)), 7),
                (GraphCell::Source, *source),
                (GraphCell::Destination, *fixed),
            ];
            row_of(fill, keys, &cells)
        })
        .collect();
    let answer = read(
        fill,
        keys,
        &rows,
        &[GraphCell::Source, GraphCell::Destination],
    );
    format!("rows {rows:?}; answer read by place {answer:?}")
}

/// The row of a `follow` of no key and of two keys, and the places of a key it has and of one it has not.
fn follow_places() -> String {
    let fill = GraphFill::FixedPoints(EDGES);
    format!(
        "no key {:?}; two keys {:?}; the second key of two {:?}; a third key of two {:?}",
        graph_row(fill, Count(0)),
        graph_row(fill, Count(2)),
        graph_column(fill, Count(2), GraphCell::Key(Count(1))),
        graph_column(fill, Count(2), GraphCell::Key(Count(2))),
    )
}

/// The rows of root 1's four visits under one key, 7, and the operation's answer (the edge, the ordinal) read by place.
fn walk_rows() -> String {
    let (fill, keys) = (GraphFill::OrderedDfs(VISITS), Count(1));
    let visits = [(1, 2, 1), (2, 3, 0), (3, 1, 0), (1, 3, 2)];
    let rows: Vec<Vec<u64>> = (0_u64..)
        .zip(visits)
        .map(|(ordinal, (source, destination, edge))| {
            let cells = [
                (GraphCell::Key(Count(0)), 7),
                (GraphCell::Root, 1),
                (GraphCell::Source, source),
                (GraphCell::Destination, destination),
                (GraphCell::Edge, edge),
                (GraphCell::Ordinal, ordinal),
            ];
            row_of(fill, keys, &cells)
        })
        .collect();
    let answer = read(fill, keys, &rows, &[GraphCell::Edge, GraphCell::Ordinal]);
    format!("rows {rows:?}; answer read by place {answer:?}")
}

/// The rows of the two edges followed from root 1, with no key, and the operation's answer (the state before, the state
/// reached) read by place.
fn worklist_rows() -> String {
    let (fill, keys) = (GraphFill::Worklist(STATES), Count(0));
    let followed = [(1, 2, 0, 0, 1), (2, 2, 0, 1, 1)];
    let rows: Vec<Vec<u64>> = followed
        .iter()
        .map(|(source, destination, order, before, reached)| {
            let cells = [
                (GraphCell::Root, 1),
                (GraphCell::Source, *source),
                (GraphCell::Destination, *destination),
                (GraphCell::Order, *order),
                (GraphCell::Before, *before),
                (GraphCell::Reached, *reached),
            ];
            row_of(fill, keys, &cells)
        })
        .collect();
    let answer = read(fill, keys, &rows, &[GraphCell::Before, GraphCell::Reached]);
    format!("rows {rows:?}; answer read by place {answer:?}")
}

/// The rows of the six entities on a cycle, and the operation's answer (the source, the destination) read by place.
fn cycle_rows() -> String {
    let (fill, keys) = (GraphFill::Cycles(RING), Count(0));
    let rows: Vec<Vec<u64>> = [1, 2, 3, 7, 8, 9]
        .iter()
        .map(|entity| {
            let cells = [
                (GraphCell::Source, *entity),
                (GraphCell::Destination, *entity),
            ];
            row_of(fill, keys, &cells)
        })
        .collect();
    let answer = read(
        fill,
        keys,
        &rows,
        &[GraphCell::Source, GraphCell::Destination],
    );
    format!("rows {rows:?}; answer read by place {answer:?}")
}

/// The row of each fill with one key, and where each row holds the ordinal of a visit and the state reached.
fn every_row() -> String {
    let fills = [
        GraphFill::FixedPoints(EDGES),
        GraphFill::OrderedDfs(VISITS),
        GraphFill::Worklist(STATES),
        GraphFill::Cycles(RING),
    ];
    let lines: Vec<String> = fills
        .iter()
        .map(|fill| {
            format!(
                "{:?}, the ordinal {:?}, the state reached {:?}",
                graph_row(*fill, Count(1)),
                graph_column(*fill, Count(1), GraphCell::Ordinal).map(|made| made.0),
                graph_column(*fill, Count(1), GraphCell::Reached).map(|made| made.0),
            )
        })
        .collect();
    lines.join("; ")
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(Cow::Owned(match scenario {
        Scenario::FollowRows => follow_rows(),
        Scenario::FollowPlaces => follow_places(),
        Scenario::WalkRows => walk_rows(),
        Scenario::WorklistRows => worklist_rows(),
        Scenario::CycleRows => cycle_rows(),
        Scenario::EveryRow => every_row(),
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
