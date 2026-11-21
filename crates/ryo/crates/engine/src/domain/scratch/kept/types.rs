//! The types of the `kept` part: what a need answered a rule, where a read column lies for a file, and how an `Exists`
//! word asks its body.

use crate::domain::code::types::{CodeEntry, NeedSite};
use crate::domain::indexes::types::FileStamp;
use crate::domain::made_tables::columns::types::HeldColumn;
use crate::domain::memos::types::RuleStamp;
use crate::domain::needs::types::NeedVerdict;
use crate::domain::steps::types::MadeTableId;
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

/// SPIKE: what the fills a need started cost, for one table, summed over a measuring worker's files.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct NeedFill {
    /// The files in which a need's decision made the table.
    pub(crate) files: Count,
    /// Of those, the files in which the table had no row: the need said the unit cannot have one.
    pub(crate) empty: Count,
    /// Of those, the files in which the table had a row and the rule running had no candidate all the same.
    pub(crate) idle: Count,
    /// The thread time of those fills, what they called with it.
    pub(crate) nanos: Count,
}

/// SPIKE: the fills needs started, by table, and the tables so made with a row while the running rule works.
#[derive(Clone, Debug, Default)]
pub(crate) struct NeedFills {
    /// By the table's number.
    pub(crate) by_table: Vec<NeedFill>,
    /// The tables a need made with a row since the running rule began.
    pub(crate) pending: Vec<MadeTableId>,
    /// The site whose decision is being traced: a rule's branch's, asked by the dispatch.
    pub(crate) tracing: Option<NeedSite>,
    /// The decisions under way inside the traced one (a table's own need): their terms are not the traced formula's.
    pub(crate) nested: Count,
    /// The terms the traced decision asked, in the order asked, each with what it said.
    pub(crate) trace: Vec<String>,
    /// How many files each site's decision asked exactly these terms with these answers.
    pub(crate) traces: std::collections::BTreeMap<String, usize>,
    /// Each traced site's rule, branch and formula, said once.
    pub(crate) formulas: std::collections::BTreeMap<u32, String>,
}
