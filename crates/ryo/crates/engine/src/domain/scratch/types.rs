//! The types of the `scratch` concept: a worker's per-file state.

#[cfg(test)]
use crate::domain::code::types::CodeEntry;
#[cfg(test)]
use crate::domain::findings::types::InstructionCount;
#[cfg(test)]
use crate::domain::indexes::types::FileStamp;
#[cfg(test)]
use crate::domain::made_tables::types::TableRows;
use ryo_language::domain::facts::types::{CellWord, FileWords, FilledColumns};
use ryo_language::domain::node_table::types::KindsPresent;

/// A worker's per-file state: registers, the filled relations and their columns, made tables, the file's words and the kinds
/// present; cleared per file, never freed. Made for a rule set by `scratch::service::scratch_for`.
///
/// ```compile_fail,E0603
/// let forged = ryo_engine::domain::scratch::types::ScanScratch(Default::default());
/// ```
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::service::scratch_for;
/// use ryo_engine::domain::programs::types::Programs;
///
/// let _scratch = scratch_for(&Programs::default());
/// ```
#[derive(Debug, Default)]
pub struct ScanScratch(pub(crate) ScratchEngine);

/// The private body of `ScanScratch`.
#[derive(Debug, Default)]
pub(crate) struct ScratchEngine {
    /// The registers of the running candidate.
    pub(crate) registers: Vec<CellWord>,
    /// Which relations the file has filled, one bit per relation of the catalog.
    pub(crate) filled: Vec<FilledWord>,
    /// The filled columns of each relation, by its number in the catalog.
    pub(crate) columns: Vec<FilledColumns>,
    /// The rows of each made table, by number, row-major.
    pub(crate) made: Vec<Vec<CellWord>>,
    /// The file's words over the run's base words.
    pub(crate) words: FileWords,
    /// The kinds present in the file.
    pub(crate) present: KindsPresent,
}

/// Sixty-four relations' filled bits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FilledWord(pub(crate) u64);

/// A place a cursor holds: a row's number in what is iterated, a kind's place in a list, or where staged rows start.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct FramePlace(pub(crate) u32);

/// The cursor of one running iteration: the word that iterates, and where it is among what it iterates. It holds places
/// in the worker's arenas, never a pointer, since the arenas grow while they are read.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct IterationCursor {
    /// The word that iterates.
    pub(crate) word: CodeEntry,
    /// The next row, as a place in what is iterated.
    pub(crate) at: FramePlace,
    /// The place after the last row.
    pub(crate) end: FramePlace,
    /// What the word needs beside: a kind's place in a list, or where its staged rows start.
    pub(crate) beside: FramePlace,
}

/// What the loop counted, added once per run of a program and never per row.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct LoopWork {
    /// The words dispatched.
    pub(crate) steps: InstructionCount,
    /// The rows the iterations gave.
    pub(crate) visited: InstructionCount,
    /// The rows a rule's anchor gave.
    pub(crate) candidates: InstructionCount,
    /// The words met that the executor does not run yet: a rule that holds one finds no row by it.
    pub(crate) unbuilt: InstructionCount,
}

/// One made table for a file.
#[cfg(test)]
#[derive(Debug, Default)]
pub(crate) struct TableSlot {
    /// The file it was made for.
    pub(crate) stamp: FileStamp,
    /// Its rows.
    pub(crate) rows: TableRows,
}

/// The loop's per-file state: what `ScanScratch` holds once the loop is the scan's body. Sized once for a rule set and
/// reset per file without an allocation: a slot stamped with another file's number is unset, every arena is emptied.
#[cfg(test)]
#[derive(Debug, Default)]
pub(crate) struct LoopState {
    /// The file's number: bumped per file.
    pub(crate) stamp: FileStamp,
    /// The registers of the rule set, one vector.
    pub(crate) registers: Vec<CellWord>,
    /// Each made table, by number.
    pub(crate) tables: Vec<TableSlot>,
    /// The cursors of the running iterations, innermost last.
    pub(crate) frames: Vec<IterationCursor>,
    /// The cells of the key or of the row being put together.
    pub(crate) key: Vec<CellWord>,
    /// The file's words over the run's base words.
    pub(crate) words: FileWords,
    /// What the loop counted.
    pub(crate) work: LoopWork,
}
