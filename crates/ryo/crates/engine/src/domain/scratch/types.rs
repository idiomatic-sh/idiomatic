//! The types of the `scratch` concept: a worker's per-file state.

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
