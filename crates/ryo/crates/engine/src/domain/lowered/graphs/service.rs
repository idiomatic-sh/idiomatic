//! The services of the `graphs` part: the columns of a graph table's row in order, the place of one of them, and the width
//! the encoding holds a graph table to. The row is stated once, here: its width and every place are read from it.

use super::types::{GraphCell, InputOrder};
use crate::domain::lowered::service::input_width;
use crate::domain::lowered::types::{FillInput, GraphFill};
use crate::domain::steps::types::MadeColumn;
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::table_words::types::ColumnCount;

/// The columns of the row of a graph table, in order, for a fill whose first input has `keys` key columns: the keys
/// first, in their order, then what the fill's kernel answers. A `cycles` has no key.
#[must_use]
pub(crate) fn graph_row(fill: GraphFill, keys: Count) -> Vec<GraphCell> {
    let keyed = (0..keys.0).map(|place| GraphCell::Key(Count(place)));
    match fill {
        GraphFill::FixedPoints(_) => keyed
            .chain([GraphCell::Source, GraphCell::Destination])
            .collect(),
        GraphFill::OrderedDfs(_) => keyed
            .chain([
                GraphCell::Root,
                GraphCell::Source,
                GraphCell::Destination,
                GraphCell::Edge,
                GraphCell::Ordinal,
            ])
            .collect(),
        GraphFill::Worklist(_) => keyed
            .chain([
                GraphCell::Root,
                GraphCell::Source,
                GraphCell::Destination,
                GraphCell::Order,
                GraphCell::Before,
                GraphCell::Reached,
            ])
            .collect(),
        GraphFill::Cycles(_) => vec![GraphCell::Source, GraphCell::Destination],
    }
}

/// The column of a graph table's row that holds `cell`, for a fill whose first input has `keys` key columns. None for a
/// cell the fill's row does not have, and for a place past what a made column numbers.
#[must_use]
pub(crate) fn graph_column(fill: GraphFill, keys: Count, cell: GraphCell) -> Option<MadeColumn> {
    let place = graph_row(fill, keys)
        .iter()
        .position(|held| *held == cell)?;
    match u16::try_from(place) {
        Ok(made) => Some(MadeColumn(made)),
        Err(_) => None,
    }
}

/// How many key columns an input has when it is one the contract states: its rows are of one width and hold `told`, its
/// keys and its order (`input_width`), and it has an order column exactly when `ordered` says so. None otherwise.
fn stated_keys(input: &FillInput, told: &[MadeColumn], ordered: InputOrder) -> Option<Count> {
    let _wide = input_width(input, told)?;
    let held = match ordered {
        InputOrder::Ordered => input.order.is_some(),
        InputOrder::Unordered => input.order.is_none(),
    };
    held.then_some(Count(input.keys.0.len()))
}

/// The key columns of the roots of a walk or a run, when its edges' keys are as many: the two lists are matched place by
/// place.
fn matched_keys(roots: Option<Count>, edges: Option<Count>) -> Option<Count> {
    roots.filter(|keys| Some(*keys) == edges)
}

/// How wide the table of a graph fill over `inputs` is: the length of its row. None when the fill is not one the contract
/// states: another number of inputs than the fill has; an input with no pipeline, or whose pipelines' rows differ in
/// width; a key, an order or a told column past its input's row; an order on an input that has none, or none on the edges
/// of a walk or a run; roots and edges whose key lists differ in length; a key on a `cycles`' edges or on a lattice. The
/// encoding writes no graph table of another width.
#[must_use]
pub(crate) fn graph_width(fill: GraphFill, inputs: &[FillInput]) -> Option<ColumnCount> {
    let none = Some(Count(0));
    let keys = match (fill, inputs) {
        (GraphFill::FixedPoints(columns), [edges]) => stated_keys(
            edges,
            &[columns.source, columns.destination],
            InputOrder::Unordered,
        ),
        (GraphFill::OrderedDfs(columns), [roots, edges]) => matched_keys(
            stated_keys(roots, &[columns.root], InputOrder::Unordered),
            stated_keys(
                edges,
                &[columns.source, columns.destination, columns.edge],
                InputOrder::Ordered,
            ),
        ),
        (GraphFill::Worklist(columns), [roots, edges, lattice]) => {
            let ends = [
                columns.source,
                columns.destination,
                columns.before,
                columns.after,
            ];
            let states = [columns.lower, columns.upper];
            let whole = stated_keys(lattice, &states, InputOrder::Unordered) == none;
            matched_keys(
                stated_keys(roots, &[columns.root, columns.state], InputOrder::Unordered),
                stated_keys(edges, &ends, InputOrder::Ordered),
            )
            .filter(|_| whole)
        }
        (GraphFill::Cycles(columns), [edges]) => stated_keys(
            edges,
            &[columns.source, columns.destination],
            InputOrder::Unordered,
        )
        .filter(|keys| Some(*keys) == none),
        (
            GraphFill::FixedPoints(_)
            | GraphFill::OrderedDfs(_)
            | GraphFill::Worklist(_)
            | GraphFill::Cycles(_),
            _,
        ) => None,
    };
    Some(ColumnCount(graph_row(fill, keys?).len()))
}
