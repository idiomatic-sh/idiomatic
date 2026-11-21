//! The types of the `graphs` part of the `lowered` concept: what a column of a graph table's row holds. Crate-private.

use ryo_relations::domain::relations::types::Count;

/// Whether an input of a graph fill names an order column: what `FillInput.order` is for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InputOrder {
    /// It names the column its rows are taken by, a number: the edges of an ordered walk and of a worklist.
    Ordered,
    /// It names none: every other input.
    Unordered,
}

/// What a column of a graph table's row holds, in the engine's own words. Which of them a fill's row has, and in what
/// order, is `graphs::service::graph_row`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GraphCell {
    /// The cell of the key at this place among the `keys` of the fill's first input.
    Key(Count),
    /// The root a walk or a run started at.
    Root,
    /// The entity an edge leaves: of a `follow`, the source a row is of; of a `cycles`, the entity on a cycle.
    Source,
    /// The entity an edge arrives at: of a `follow`, the fixed point the source reaches; of a `cycles`, the entity again.
    Destination,
    /// The identity of the edge an ordered walk visited.
    Edge,
    /// The place of a visit among its root's, a number from 0.
    Ordinal,
    /// The order of the edge a worklist followed.
    Order,
    /// The state the edge's source held when a worklist followed it.
    Before,
    /// The state the edge's destination holds after the edge's join.
    Reached,
}
