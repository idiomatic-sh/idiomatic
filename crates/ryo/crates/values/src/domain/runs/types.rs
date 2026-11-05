//! The types of the `runs` concept: what a run is asked, and the names it counts in. Plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::descriptions::types::LanguageName;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::reasons::types::ToolMessage;
use ryo_language::inbound::use_cases::project::types::RootPath;

/// How much memory a run may use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Budget {
    /// What the machine has.
    Machine,
    /// This many bytes, at most what the machine has.
    Bytes(MemoryBytes),
}

/// An amount of memory, in bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct MemoryBytes(pub u64);

/// How many workers the caller asks to read files with; zero asks for as many as the machine has.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorkersAsked(pub usize);

/// A repository opened for linting: where it is and what the run may use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Session {
    /// The repository's root directory.
    pub root: RootPath,
    /// Where its files are: on the disk below the root, or only those given.
    pub repository: ryo_language::inbound::use_cases::project::types::Repository,
    /// Explicit ordered project import roots, or an undeclared scope whose absence is unknown.
    pub search_scope: ryo_language::domain::project_context::types::ProjectSearchScope,
    /// Host authority for candidate entries when repository files are Given.
    pub given_inventory: ryo_language::domain::project_context::types::GivenProjectInventory,
}

/// The id of a rule, as the store keeps it.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuleId(pub Box<str>);

/// Whether a rule runs regardless of parser acceptance or only on accepted syntax trees.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RuleApplicability {
    /// Run the rule on every admitted file, as legacy rules do.
    #[default]
    Always,
    /// Run only when the language provider explicitly accepted the file's syntax.
    AcceptedSyntax,
}

/// Why a rule was explicitly skipped for one file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SkipReason {
    /// The language provider explicitly rejected the file's syntax.
    SyntaxRejected,
}

/// A rule that did not run for a file because its applicability condition was not met.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Skipped {
    /// The rule.
    pub rule: RuleId,
    /// The file.
    pub path: FilePath,
    /// Why it was skipped.
    pub reason: SkipReason,
}

/// A manifest a language's tools refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManifestRefused {
    /// The language.
    pub language: LanguageName,
    /// The manifest's path, relative to the root.
    pub path: FilePath,
    /// What the tool said.
    pub reason: ToolMessage,
}

/// How many workers the machine has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MachineWorkers(pub u32);

/// How many workers a run starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StartedWorkers(pub u32);

/// What a lint writes for each row a rule emits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnswerShape {
    /// A finding: the rule, the path, the key, every column as one cell, the line and the character; rows that give the same rule,
    /// path and key are one, and every finding is written. The run keeps what it found for the next.
    Findings,
    /// The rule's own columns, then the line and the character: every row, in the order of the files and, in a file, of the rules
    /// and the places, the rows of the cross-file steps last. With more than one rule, the rule's id comes first and the columns are
    /// every rule's, each empty in the rows of a rule that has none of that heading. At most this many rows are written, and every
    /// row is counted. Nothing is kept for the next run.
    Rows(crate::domain::records::types::RowTotal),
}

/// Whether the common file DAG may retain optional completed node results.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NodeResultReuse {
    /// Required live consumer leases remain; optional completed node retention is disabled.
    #[default]
    Disabled,
    /// Admit optional completed node results under the existing bounded policy.
    Enabled,
}

/// Physical execution choices selected at the caller edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExecutionOptions {
    /// Optional node result retention; does not select a different executor.
    pub node_result_reuse: NodeResultReuse,
    /// Enable eligible column kernels.
    pub column_kernel: bool,
}

/// A rule, by its place among the rules of a run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuleNumber(pub u32);
