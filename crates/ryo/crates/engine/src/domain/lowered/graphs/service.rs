//! The services of the `graphs` part: the columns of a graph table's row in order, the place of one of them, and the width
//! the encoding holds a graph table to. The row is stated once, here: its width and every place are read from it.

use super::types::GraphCell;
use crate::domain::lowered::service::input_width;
use crate::domain::lowered::types::{FillInput, GraphFill};
use crate::domain::steps::types::MadeColumn;
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::table_words::types::ColumnCount;

/// The columns of the row of a graph table, in order, for a fill whose first input has `keys` key columns. A fill whose
/// row the contract does not state has no column.
#[must_use]
pub(crate) fn graph_row(fill: GraphFill, keys: Count) -> Vec<GraphCell> {
    let keyed = (0..keys.0).map(|place| GraphCell::Key(Count(place)));
    match fill {
        GraphFill::FixedPoints(_) => keyed
            .chain([GraphCell::Source, GraphCell::Destination])
            .collect(),
        GraphFill::OrderedDfs | GraphFill::Worklist | GraphFill::Cycles => Vec::new(),
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

/// How wide the table of a graph fill over `inputs` is: the length of its row. None when the fill is not one the contract
/// states: another number of inputs than the fill has; an input with no pipeline, or whose pipelines' rows differ in
/// width; a key, an order or a told column past its input's row; an order on an input that has none. The encoding writes
/// no graph table of another width.
#[must_use]
pub(crate) fn graph_width(fill: GraphFill, inputs: &[FillInput]) -> Option<ColumnCount> {
    let keys = match (fill, inputs) {
        (GraphFill::FixedPoints(columns), [edges]) => {
            let _wide = input_width(edges, &[columns.source, columns.destination])?;
            edges.order.is_none().then_some(edges.keys.0.len())?
        }
        (
            GraphFill::FixedPoints(_)
            | GraphFill::OrderedDfs
            | GraphFill::Worklist
            | GraphFill::Cycles,
            _,
        ) => return None,
    };
    Some(ColumnCount(graph_row(fill, Count(keys)).len()))
}
