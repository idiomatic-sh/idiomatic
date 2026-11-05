//! The types of the `coverage` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_relations::domain::relations::types::{FileId, Relation};

use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::stored_facts::types::PartialCause;

/// Which rows of a relation could be missing from a file that is read in part.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Lacking {
    /// Any row could be.
    #[default]
    Any,
    /// Only rows of this `kind` could be.
    Kind(LackingKind),
}

/// The kind of the rows that could be missing, as the reader names it.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct LackingKind(pub Box<str>);

/// A relation a file is read in part, why, and which rows could be missing.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Partial {
    /// The relation.
    pub relation: Relation,
    /// Why, as the reader says it.
    pub cause: PartialCause,
    /// Which rows could be missing.
    pub lacking: Lacking,
    /// The file the rows missing are about; none when it is not said.
    pub at: Option<FileId>,
}

/// The relations of the files of a run that are read only in part, by file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Partials {
    /// Each partly read relation of each file, in no particular order.
    pub entries: Vec<FilePartial>,
    /// Files explicitly skipped for a rule before its cross-file steps; these must not look like clean empty inputs.
    pub unavailable_files: Vec<FileId>,
}

/// A relation read in part, and the file it is read in part in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilePartial {
    /// The file; none for the repository as a whole.
    pub file: Option<FileId>,
    /// The relation read in part there.
    pub partial: Partial,
}

/// The place of a rule among the rules of a run: an index into their needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuleIndex(pub usize);

/// What is partial in a file's relation, and why, for a human.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Message(pub Box<str>);

/// Something a rule could not decide, and the file that is the reason.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Unknown {
    /// The rule that could not decide.
    pub rule: RuleId,
    /// The file whose relation is read only in part.
    pub path: FilePath,
    /// What is partial there and why.
    pub message: Message,
}

/// Where a relation is read in part: one file, or the repository as a whole.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoverageScope<'a> {
    /// The repository as a whole.
    Repository,
    /// This file.
    File(&'a FilePath),
}

/// Whether a column of a relation holds the files, or the nodes of files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Keying {
    /// It holds files or nodes: a row says which file it is in.
    ByFile,
    /// It holds something else.
    NotByFile,
}

/// A file of which some relations are read in part: its place among the files, the rules that govern it and what is partial.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Covered {
    /// The file's place among the files of the run.
    pub file: FileId,
    /// The rules that govern the file, by their place.
    pub rules: Vec<RuleIndex>,
    /// Each partly read relation.
    pub partial: Vec<Partial>,
}

/// How much of a file is only a recovery of what was written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recovery {
    /// None: the file was read as it is.
    Whole,
    /// The tree is a recovery from syntax errors.
    SyntaxErrors,
    /// The AST was recovered, but lexical tokenization is complete.
    LexicallyCompleteSyntaxErrors,
    /// No language could parse the file.
    Unparsed,
}

/// Actual root readiness for one immutable request file generation.
#[derive(Clone, Debug)]
pub struct FileObservationReadiness {
    pub file: FileId,
    pub root: std::sync::Arc<ryo_values::domain::records::types::FrontierReadiness>,
}
