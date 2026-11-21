//! The rows of the graph tables, each stated for a small input as a case of the operators crate has it, every cell written
//! at the column `graph_column` gives it and the operation's own answer read back by place. Then which fills the
//! contract states: the width `graph_width` answers, and each way a fill is not one.
//!
//! `follow`: one key, 7, with the edges (1, 2), (2, 3), (3, 3) of the case
//! `each_source_answers_its_fixed_point_and_a_self_pointer_answers_itself`, whose pairs are (1, 3), (2, 3), (3, 3).

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::records::types::EdgeColumns;
use crate::domain::lowered::graphs::service::{graph_column, graph_row, graph_width};
use crate::domain::lowered::graphs::types::GraphCell;
use crate::domain::lowered::types::{FillInput, GraphFill};
use crate::domain::steps::stages::types::Pipeline;
use crate::domain::steps::types::{ColumnList, MadeColumn, Register, StepOperand, StepOperands};
use ryo_relations::domain::relations::types::Count;
use std::borrow::Cow;

/// What a row asks.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// The rows of a `follow` table for the small input, and the operation's answer read from them by place.
    FollowRows,
    /// The width of a `follow` as the contract states it, and of each way a fill is not one.
    FollowStated,
    /// The row of a `follow` of no key and of two, and the place of a cell its row does not have.
    FollowPlaces,
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
            "a_follow_is_stated_by_its_edges_alone_unordered_and_a_table_two_wider_than_its_keys",
        ),
        word: Word::Succeeds,
        input: || Scenario::FollowStated,
        expect: Expect::Value(Cow::Borrowed(
            "stated Some(3); no key Some(2); ordered edges None; a destination past its row None; a key past its row None; two inputs None; no input None; pipelines of two widths None; no pipeline None",
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
];

/// The edge columns of the small inputs: an edge row is its key, its source, its destination.
const EDGES: EdgeColumns = EdgeColumns {
    source: MadeColumn(1),
    destination: MadeColumn(2),
};

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

/// An input of one pipeline whose rows are `width` wide, keyed on `keys`, ordered by `order`.
fn input(width: u32, keys: &[u16], order: Option<u16>) -> FillInput {
    FillInput {
        pipelines: Box::new([pipeline(width)]),
        keys: ColumnList(keys.iter().copied().map(MadeColumn).collect()),
        order: order.map(MadeColumn),
    }
}

/// A pipeline of no stage whose rows are `width` wide.
fn pipeline(width: u32) -> Pipeline {
    Pipeline {
        stages: Box::new([]),
        row: StepOperands(
            (0..width)
                .map(|at| StepOperand::Register(Register(at)))
                .collect(),
        ),
    }
}

/// The width of each `follow`, the stated one first.
fn follow_stated() -> String {
    let width = |fill, inputs: &[FillInput]| graph_width(fill, inputs).map(|width| width.0);
    let fill = GraphFill::FixedPoints(EDGES);
    let mut uneven = input(3, &[0], None);
    uneven.pipelines = Box::new([pipeline(3), pipeline(4)]);
    let mut empty = input(3, &[0], None);
    empty.pipelines = Box::new([]);
    let past = GraphFill::FixedPoints(EdgeColumns {
        source: MadeColumn(1),
        destination: MadeColumn(3),
    });
    format!(
        "stated {:?}; no key {:?}; ordered edges {:?}; a destination past its row {:?}; a key past its row {:?}; two \
         inputs {:?}; no input {:?}; pipelines of two widths {:?}; no pipeline {:?}",
        width(fill, &[input(3, &[0], None)]),
        width(fill, &[input(3, &[], None)]),
        width(fill, &[input(3, &[0], Some(1))]),
        width(past, &[input(3, &[0], None)]),
        width(fill, &[input(3, &[3], None)]),
        width(fill, &[input(3, &[0], None), input(3, &[0], None)]),
        width(fill, &[]),
        width(fill, &[uneven]),
        width(fill, &[empty]),
    )
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

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(Cow::Owned(match scenario {
        Scenario::FollowRows => follow_rows(),
        Scenario::FollowStated => follow_stated(),
        Scenario::FollowPlaces => follow_places(),
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
