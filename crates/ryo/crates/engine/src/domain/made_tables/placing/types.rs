//! The types of the `placing` part: the room the `Group(Places)` fill works in.

use crate::domain::indexes::types::SourceRow;
use ryo_operators::domain::ordered_groups::types::Place;

/// The room the `Group(Places)` fill works in: one key's distinct rows and their places. Kept between fills, never
/// freed.
#[derive(Debug, Default)]
pub(crate) struct PlaceRoom {
    /// One key's rows, ascending by their order, a repeated row once.
    pub(crate) rows: Vec<SourceRow>,
    /// The place of each of them, as the meaning wrote it.
    pub(crate) places: Vec<Place>,
}
