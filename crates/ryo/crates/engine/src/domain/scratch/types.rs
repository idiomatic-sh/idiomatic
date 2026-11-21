//! The types of the `scratch` concept: a worker's per-file state.

use crate::domain::code::types::CodeEntry;
use crate::domain::filled::errors::FilledError;
use crate::domain::findings::types::InstructionCount;
use crate::domain::indexes::types::RowRun;
use crate::domain::made_tables::types::FillStore;
use crate::domain::memos::types::{MemoId, MemoMap, RuleStamp};
use crate::domain::needs::types::NeedVerdict;
use crate::domain::stages::types::{BatchRow, FanRows};
use ryo_language::domain::facts::types::{CellWord, FileWords};
use ryo_operators::domain::table_scratch::types::Patterns;
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// A worker's per-file state: the loop's registers, kept answers and cursors, the fill side's store of filled relations,
/// indexes and made tables, and the file's words; reset per file, never freed. Made for a rule set by
/// `programs::service::scratch_for`.
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

/// The private body of `ScanScratch`: the loop's own state, the fill side's store of what was filled and made, and the
/// file's words. The store and the words are lent to the fill side for each call of its seam; the state is the loop's.
#[derive(Debug, Default)]
pub(crate) struct ScratchEngine {
    /// The loop's own per-file state.
    pub(crate) state: LoopState,
    /// The fill side's store.
    pub(crate) store: FillStore,
    /// The file's words over the run's base words.
    pub(crate) words: FileWords,
}

/// A place a cursor holds: a row's number in what is iterated, a kind's place in a list, or where staged rows start.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct FramePlace(pub(crate) u32);

/// The cursor of one running iteration: the word that iterates, and where it is among what it iterates. It holds places
/// in the worker's arenas, never a pointer, since the arenas grow while they are read.
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

/// What a body answered the word over it: how many rows, and for an `Or` where they start among the staged rows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct BodyAnswer {
    /// Where its rows start among the staged rows.
    pub(crate) first: Count,
    /// How many rows it has.
    pub(crate) rows: Count,
}

/// A fault the fill side answered, and the rule that was running: the scan's answer for the file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FileFault {
    /// The rule.
    pub(crate) rule: SelectedRuleOrdinal,
    /// The fault.
    pub(crate) fault: FilledError,
}

/// What the source of the word about to run answered: the rows it has for the word, as the seam's run, and what turns one
/// of its rows into the cell a register holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SeamRows {
    /// The rows: a range of handles for a whole read, a stretch of the store's rows for a read by key.
    pub(crate) run: RowRun,
    /// What is or-ed onto a row to make its handle: nought, or the file's half of a node's id.
    pub(crate) tag: CellWord,
}

/// The loop's per-file state: what `ScanScratch` holds once the loop is the scan's body. Sized once for a rule set and
/// reset per file without an allocation: every arena is emptied. The fill side's store and the file's words are beside it, in
/// the frame's other half.
#[derive(Debug, Default)]
pub(crate) struct LoopState {
    /// The registers of the rule set, one vector.
    pub(crate) registers: Vec<CellWord>,
    /// The answers kept for the file.
    pub(crate) memos: MemoMap,
    /// The cursors of the running iterations, innermost last.
    pub(crate) frames: Vec<IterationCursor>,
    /// The rows the running bodies yielded, each body's above its caller's.
    pub(crate) yielded: Vec<CellWord>,
    /// The rows each running `Or` iterates, innermost last.
    pub(crate) staged: Vec<CellWord>,
    /// What the source of the word about to run answered: the loop asks the fill side, the word's item reads the rows.
    pub(crate) rows: SeamRows,
    /// The regular expressions compiled so far, by source: kept across files.
    pub(crate) patterns: Patterns,
    /// The selection a test of a text writes for its batch of one row.
    pub(crate) kept: Vec<BatchRow>,
    /// The room a walk of the tree writes what it reaches in, lent to the fill side's kernel.
    pub(crate) fan: FanRows,
    /// The rule whose work is running; none before a file's first rule.
    pub(crate) rule: Option<SelectedRuleOrdinal>,
    /// The first fault the fill side answered in the file, a relation refused or read in part, and the rule that was
    /// running.
    pub(crate) fault: Option<FileFault>,
    /// What the body of the word about to run answered: the loop asks the body, the word's item reads the answer.
    pub(crate) answer: BodyAnswer,
    /// Each emptiness decision made in the file, by `NeedSite`.
    pub(crate) needs: Vec<Option<NeedVerdict>>,
    /// The cells of the key or of the row being put together.
    pub(crate) key: Vec<CellWord>,
    /// What the loop counted.
    pub(crate) work: LoopWork,
    /// What the running rule did and was charged in the file.
    pub(crate) budget: RuleBudget,
}

/// What one unit of the code was charged to the running rule: how many of its keys, and the rows they visited.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct UnitTally {
    /// The rule the tally is of: a tally of another rule is nought.
    pub(crate) stamp: RuleStamp,
    /// The keys of the unit charged.
    pub(crate) keys: InstructionCount,
    /// The rows their programs visited.
    pub(crate) visited: InstructionCount,
}

/// What the running rule did in the file and what it was charged: counted in rows, never in time. A unit records its own
/// work and the list of what it reached; the rule is charged each unit it reaches once, however it was reached, so its
/// work is the same alone or after any other rule.
#[derive(Debug, Default)]
pub(crate) struct RuleBudget {
    /// The running rule's run over the file.
    pub(crate) stamp: RuleStamp,
    /// The rows the running unit's own iterations gave: the rule's own program, or the pattern being evaluated.
    pub(crate) visited: InstructionCount,
    /// The rule's candidates.
    pub(crate) candidates: InstructionCount,
    /// How many patterns are being evaluated, one inside the other.
    pub(crate) depth: Count,
    /// The answers the patterns being evaluated asked directly, each pattern's above its caller's.
    pub(crate) reaching: Vec<MemoId>,
    /// The work of the units charged to the rule.
    pub(crate) charged: InstructionCount,
    /// The keys of the call patterns charged to the rule.
    pub(crate) keys: InstructionCount,
    /// What each unit was charged to the rule, by the unit's number.
    pub(crate) tallies: Vec<UnitTally>,
    /// The answers still to charge, while a list is followed.
    pub(crate) pending: Vec<MemoId>,
}
