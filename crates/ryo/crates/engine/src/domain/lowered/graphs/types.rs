//! The types of the `graphs` part of the `lowered` concept: what a column of a graph table's row holds. Crate-private.

use ryo_relations::domain::relations::types::Count;

/// What a column of a graph table's row holds, in the engine's own words. Which of them a fill's row has, and in what
/// order, is `graphs::service::graph_row`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GraphCell {
    /// The cell of the key at this place among the `keys` of the fill's first input.
    Key(Count),
    /// The entity an edge leaves: of a `follow`, the source a row is of.
    Source,
    /// The entity an edge arrives at: of a `follow`, the fixed point the source reaches.
    Destination,
}
