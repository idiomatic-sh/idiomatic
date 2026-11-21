//! The types of the `kept` part: what a need answered a rule, where a read column lies for a file, and how an `Exists`
//! word asks its body.

use crate::domain::code::types::CodeEntry;
use crate::domain::indexes::types::FileStamp;
use crate::domain::made_tables::columns::types::HeldColumn;
use crate::domain::memos::types::RuleStamp;
use crate::domain::needs::types::NeedVerdict;
use ryo_relations::domain::relations::types::Count;

/// What a definition's need answered a rule's ask in a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AskedNeed {
    /// The rule's run over the file that asked; another's, or none, before this rule's first ask in the file.
    pub(crate) rule: RuleStamp,
    /// What the need said.
    pub(crate) verdict: NeedVerdict,
}

/// A column a `Read` word reads, as the fill side holds it for one file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HeldRead {
    /// The file it was asked in; none before the word's first read.
    pub(crate) file: Option<FileStamp>,
    /// Where the column lies.
    pub(crate) column: HeldColumn,
}

/// How an `Exists` word that keeps no answer of its own asks its body: decided from the code when the worker's state
/// is made.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum QuickBody {
    /// As any body is asked: a table read, a pattern whose answers are kept, a word that keeps its own.
    #[default]
    Kept,
    /// An inline body of one `Yield`: it has a row whatever the candidate, and nothing is run.
    Row,
    /// An inline body: its program is run where the word stands.
    Inline(CodeEntry),
    /// A pattern in call form whose program is a short straight line (`STRAIGHT_WORDS`): running it costs less than
    /// keeping its answer, so it is run where the word stands, as an inline body is, and no answer is kept.
    Straight(CodeEntry),
}

/// SPIKE: what a rule's asks of one memo site came to in a file: how many it made and how many found a kept answer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SiteAsks {
    /// The rule's run over the file these are of.
    pub(crate) rule: RuleStamp,
    /// The asks made.
    pub(crate) asks: Count,
    /// The asks a kept answer answered.
    pub(crate) hits: Count,
}

/// What a register holds, as far as a test of a text must know: a cell that is a word of the file, or the file itself,
/// whose text is its path.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum RegisterHolds {
    /// A word, a number, a handle: read as it is.
    #[default]
    Cells,
    /// The file: some word that writes the register writes the file into it.
    TheFile,
}
