//! The types of the `budget` part.

use crate::domain::findings::types::InstructionCount;
use crate::domain::made_tables::types::FillStore;
use ryo_language::domain::node_table::types::NodeTable;
use ryo_relations::domain::relations::types::Count;

/// What a pattern's evaluation set aside of its caller: the caller's own rows so far, and where its own list starts.
#[derive(Clone, Copy, Debug)]
pub(crate) struct UnitCaller {
    /// The rows the caller's own iterations gave so far.
    pub(crate) visited: InstructionCount,
    /// Where the pattern's list of asked answers starts.
    pub(crate) asked: Count,
}

/// What says how much a file holds, for a rule's allowance and its relations' sizes: the fill side's store and the
/// file's node table.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FileSizes<'a> {
    /// The store.
    pub(crate) store: &'a FillStore,
    /// The node table.
    pub(crate) nodes: &'a NodeTable,
}
