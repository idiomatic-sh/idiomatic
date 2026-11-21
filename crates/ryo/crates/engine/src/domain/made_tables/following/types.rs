//! The types of the `following` part: the room the `Graph(FixedPoints)` fill works in.

use ryo_language::domain::facts::types::CellWord;
use ryo_operators::domain::follow::types::Pointer;

/// The room the `Graph(FixedPoints)` fill works in: one key's entities, its edges as the meaning reads them, and the
/// pairs the meaning answered. Kept between fills, never freed.
#[derive(Debug, Default)]
pub(crate) struct FollowRoom {
    /// The cells of one key's entities, ascending: an entity's number for the meaning is its place here.
    pub(crate) entities: Vec<CellWord>,
    /// One key's edges, by the entities' numbers, sorted by source.
    pub(crate) edges: Vec<Pointer>,
    /// The pairs of a source and a fixed point it reaches.
    pub(crate) pairs: Vec<Pointer>,
}
