//! The types of the `staging` part: what one stage reads besides its batch.

use crate::domain::made_tables::types::FillStore;
use crate::domain::programs::types::ProgramsEngine;
use ryo_language::domain::node_table::types::NodeTable;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::types::FileId;

/// What a stage reads besides its batch: the programs, the fill side's state for the file as it is (nothing is made or
/// filled while a stage runs), the file's nodes and the file.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BatchFrame<'s> {
    /// The rule set's programs.
    pub(crate) programs: &'s ProgramsEngine,
    /// The fill side's state for the file.
    pub(crate) store: &'s FillStore,
    /// The file's nodes.
    pub(crate) nodes: &'s NodeTable,
    /// The file.
    pub(crate) file: FileId,
    /// The file's bytes.
    pub(crate) text: ByteSlice<'s>,
}
