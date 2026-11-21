//! The types of the `staging` part: what one stage reads besides its batch.

use crate::domain::made_tables::types::{FillStore, RuleSetView};
use ryo_language::domain::facts::types::CellWord;
use ryo_language::domain::node_table::types::NodeTable;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_operators::domain::patterns::types::PatternMatcher;
use ryo_operators::domain::table_scratch::types::Patterns;
use ryo_relations::domain::relations::types::FileId;

/// What a stage reads besides its batch: the programs, the fill side's state for the file as it is (nothing is made or
/// filled while a stage runs), the file's nodes and the file.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BatchFrame<'s> {
    /// What the fill side reads of the rule set.
    pub(crate) programs: RuleSetView<'s>,
    /// The fill side's state for the file.
    pub(crate) store: &'s FillStore,
    /// The file's nodes.
    pub(crate) nodes: &'s NodeTable,
    /// The file.
    pub(crate) file: FileId,
    /// The file's bytes.
    pub(crate) text: ByteSlice<'s>,
}

/// What a `Keep` works in besides its batch: the cells of a set, the expressions compiled so far, and the matcher.
#[derive(Debug)]
pub(crate) struct KeepScratch<'r> {
    /// The cells of a set the test is against.
    pub(crate) set: &'r mut Vec<CellWord>,
    /// The regular expressions compiled so far.
    pub(crate) compiled: &'r mut Patterns,
    /// The matcher.
    pub(crate) matcher: PatternMatcher<'r>,
}
