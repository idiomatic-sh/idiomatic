//! The types of the `memos` concept: an answer kept per file by the site that asks and the cells of its key. Crate-private.

use crate::domain::code::types::{MemoSite, WordSlot};
use crate::domain::findings::types::InstructionCount;
use crate::domain::made_tables::types::Reached;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// A worker's kept answers: emptied per file, never freed.
#[derive(Debug, Default)]
pub(crate) struct MemoMap {
    /// Every answer kept for the file, in the order kept.
    pub(crate) entries: Vec<MemoEntry>,
    /// The cells of every key and of every answer's rows.
    pub(crate) cells: Vec<CellWord>,
    /// The answers by site and key.
    pub(crate) found: hashbrown::HashTable<MemoId>,
    /// What each answer of a pattern cost and reached, by the answer's number: what a rule that reaches it is charged.
    pub(crate) charges: Vec<EntryCharge>,
    /// The answers each answer asked directly while it was evaluated, one list after the other.
    pub(crate) reached: Vec<ChargedUnit>,
}

/// A stretch of the kept cells.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CellSpan {
    /// Where it starts.
    pub(crate) start: Count,
    /// How many cells it has.
    pub(crate) length: Count,
}

/// One kept answer: who asked, with which key, and the rows answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MemoEntry {
    /// The site that asked: a pattern, or a step over an inline body.
    pub(crate) site: MemoSite,
    /// The cells of the key.
    pub(crate) key: CellSpan,
    /// The cells of the rows answered, row by row.
    pub(crate) rows: CellSpan,
    /// How many rows were answered: an answer of rows with no cell has rows too.
    pub(crate) count: Count,
}

/// A kept answer, by its number among the file's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct MemoId(pub(crate) u32);

/// A rule's run over a file, by number: an answer stamped with it has been charged to that rule in that file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct RuleStamp(pub(crate) u32);

/// What an answer of a pattern cost and reached. It holds its own program's work alone, never the work of what it
/// reached: a rule that reaches it is charged it once, and follows the list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct EntryCharge {
    /// The rows its own program's iterations gave.
    pub(crate) work: InstructionCount,
    /// The rule last charged for it.
    pub(crate) charged: RuleStamp,
    /// The units it reached directly, as a stretch of the map's `reached`.
    pub(crate) reached: CellSpan,
    /// The unit it is of, among the code's units.
    pub(crate) unit: WordSlot,
}

/// A unit a rule's work reached directly: an answer of a pattern, or something the fill side holds (a made table, a
/// relation, an index, the node table).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChargedUnit {
    /// An answer of a pattern.
    Answer(MemoId),
    /// What the fill side holds.
    Fill(Reached),
}
