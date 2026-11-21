//! The types of the `budget` part.

use crate::domain::findings::types::InstructionCount;
use ryo_relations::domain::relations::types::Count;

/// What a pattern's evaluation set aside of its caller: the caller's own rows so far, and where its own list starts.
#[derive(Clone, Copy, Debug)]
pub(crate) struct UnitCaller {
    /// The rows the caller's own iterations gave so far.
    pub(crate) visited: InstructionCount,
    /// Where the pattern's list of asked answers starts.
    pub(crate) asked: Count,
}
