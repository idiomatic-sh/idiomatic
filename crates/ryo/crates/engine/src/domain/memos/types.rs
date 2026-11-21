//! The types of the `memos` concept: an answer kept per file by the site that asks and the cells of its key. Crate-private.

use crate::domain::code::types::MemoSite;
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
