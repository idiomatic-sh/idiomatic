//! The types of `linter lint`: what a caller states and what comes back. They are the SDK's own: nothing of the core is in them.

use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;

use ryo_core::outbound::language::interface::LanguagePort;
use ryo_core::outbound::matching::interface::MatchingPort;
use ryo_core::outbound::query_language::interface::QueryLanguagePort;
use ryo_core::outbound::scheduler::{interface::SchedulerPort, types::WorkerCount};

use crate::domain::capacity::types::MemoryBudget;
use crate::domain::cell::types::{Cell, Slot};
use crate::domain::file::types::{RelativePath, Unread};
use crate::domain::host::types::{HostColumn, HostRelationName};
use crate::domain::query::types::RyoQlQuery;
use crate::domain::relation::types::ColumnName;
use ryo_values::domain::files::types::{CorpusFile, CorpusListing};
use ryo_values::domain::runs::types::RuleApplicability;

/// The name a rule's findings carry: text that is not blank. Made through `RuleId::try_from`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuleId(Box<str>);

// ignore: idiomatic.arch/idiomatic-rust/no-bare-text - the newtype's own text crosses only to its sibling schemas, which check it on the way in and read it through `AsRef<str>`; the field stays private so no caller outside them makes an unchecked one, which a `pub(super)` field would allow its whole module
impl RuleId {
    /// A rule id of `text`, which the core holds or a schema has checked: what the sibling schemas build, never a caller.
    pub(super) const fn from_text(text: Box<str>) -> Self {
        Self(text)
    }
}

impl std::fmt::Display for RuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for RuleId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A glob over the paths of a repository, relative to its root: text that parses as one. Made through `Glob::try_from`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Glob(Box<str>);

// ignore: idiomatic.arch/idiomatic-rust/no-bare-text - the newtype's own text crosses only to its sibling schemas, which check it on the way in and read it through `AsRef<str>`; the field stays private so no caller outside them makes an unchecked one, which a `pub(super)` field would allow its whole module
impl Glob {
    /// A glob of `text`, which a schema has checked: what the sibling schemas build, never a caller.
    pub(super) const fn from_text(text: Box<str>) -> Self {
        Self(text)
    }
}

impl std::fmt::Display for Glob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for Glob {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// What distinguishes a finding from the rule's other findings in the same file: the value of each of the rule's key columns, in
/// order, typed; none for a value that reads as nothing.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FindingKey(pub Vec<Slot>);

/// What a rule could not decide, in a sentence.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Message(pub Box<str>);

/// The name of a language, as it is linked.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProviderName(pub Box<str>);

impl std::fmt::Display for ProviderName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// How many workers read files at most; the machine's own count when it has fewer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Workers(pub NonZeroUsize);

/// Which files a run reads, as the corpus adapters take them.
#[derive(Debug)]
pub(crate) enum Reading {
    /// From the disk, walked or as listed, with the files given beside it.
    Disk(CorpusListing, Vec<CorpusFile>),
    /// Only the files given as text.
    Memory(Vec<CorpusFile>),
}

/// Where a run keeps what it leaves for the next. The repository is never written to unless the caller names it.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Cache {
    /// Nothing is kept: the run reads every file.
    None,
    /// Under this directory, which the caller owns.
    At(PathBuf),
}

/// One rule: a query whose rows are violations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    /// The RyoQL text.
    pub query: RyoQlQuery,
    /// Whether the rule runs on every parse or only files whose syntax the provider accepted.
    pub applicability: RuleApplicability,
    /// What the query's parameters are bound to, by name.
    pub parameters: BTreeMap<ParameterName, ParameterValue>,
    /// The files the rule governs; none means every file.
    pub include: Vec<Glob>,
    /// The files it does not govern.
    pub exclude: Vec<Glob>,
}

/// Where a run spills what does not fit in memory while it runs.
#[derive(Debug)]
pub(crate) enum Spill {
    /// In the cache directory the caller owns.
    Kept(PathBuf),
    /// In the system's temporary directory, for a run that keeps nothing: the spill adapter leaves nothing there.
    Scratch,
}

/// The engine a program builds once, with the languages it linked, and lints many repositories with: the compiler of
/// queries, the plans compiled so far and the globs made so far are shared by every run over it. Made by `linter`.
pub struct Linter(LinterLintEngine);

/// What every run over one linter shares: the directories the linked languages skip, the language port over them, the
/// compiler of queries, the matching, the plans compiled so far, the build that runs and where runs that keep nothing spill.
pub(crate) struct LinterLintEngine {
    /// The directories every walk skips.
    pub skipped: ryo_core::outbound::corpus::types::SkippedDirectories,
    /// The linked languages, behind the language port.
    pub language: Arc<dyn LanguagePort>,
    /// The compiler of queries.
    pub query_language: Arc<dyn QueryLanguagePort>,
    /// The matching every run checks its globs with: a set made once is not made again.
    pub matching: Arc<dyn MatchingPort>,
    /// The plans compiled so far, by rule id.
    pub plans: ryo_core::inbound::use_cases::lint::types::CompiledPlans,
    /// The scheduler over the pool the caller built and injected once: every run's work is spread over it.
    pub scheduler: Arc<dyn SchedulerPort>,
    /// How many workers the scheduler runs jobs on.
    pub workers: WorkerCount,
    /// The build of the program and of its languages, which a kept store is for.
    pub build: ryo_core::outbound::lint_store::types::BuildIdentity,
    /// The most memory a run may use.
    pub memory: MemoryBudget,
    /// Where the runs keep what they leave for the next.
    pub cache: Cache,
}

impl Linter {
    /// The linter over `engine`, which its sibling entry builds, never a caller.
    pub(super) const fn over(engine: LinterLintEngine) -> Self {
        Self(engine)
    }

    /// The engine it runs over, for its sibling entry.
    pub(super) const fn engine(&self) -> &LinterLintEngine {
        &self.0
    }
}

impl std::fmt::Debug for Linter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

/// The rules to check in a repository.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LintRequest {
    /// The repository's root directory.
    pub root: PathBuf,
    /// The rules, by the id their findings carry.
    pub rules: BTreeMap<RuleId, Rule>,
    /// Only findings in these files are reported; none means every file.
    pub files: Vec<Glob>,
    /// The relations the host declares, with their rows, which a rule names as it names ryo's own; checked before the run.
    pub hosted: Vec<HostRelation>,
    /// Which files the run reads.
    pub corpus: Corpus,
    /// Files given as text, read beside the corpus: one at a path the corpus lists is read as given, in place of the disk's.
    pub beside: Vec<CaseSource>,
}

/// Which files a run reads.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Corpus {
    /// The files below the root the repository does not ignore, found by walking it.
    Walked,
    /// The files the host listed, relative to the root: the run walks nothing and reads no other file.
    Listed(Vec<RelativePath>),
    /// Only these files, given as text: the files of a rule's case. The root names nothing then.
    Given(Vec<CaseSource>),
}

/// The place of a row among the rows of a relation the host declares, from one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowNumber(pub usize);

/// The text of a file given with the request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceText(pub Box<str>);

/// A file given as text, for the run to read instead of the disk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseSource {
    /// The file, relative to the root.
    pub path: RelativePath,
    /// What it holds.
    pub text: SourceText,
}

/// One violation of a rule.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Finding {
    /// The rule that found it.
    pub rule: RuleId,
    /// What it is about: the value of the rule's subject column, typed (a file, for a rule about a file); nothing for a value that
    /// reads as nothing.
    pub subject: Slot,
    /// What distinguishes it from the rule's other findings in the file.
    pub key: FindingKey,
    /// Where in the file the rule's primary cell puts it.
    pub place: Place,
    /// Every column of the row the query emitted, in order, each value typed.
    pub columns: Vec<FindingColumn>,
}

/// One column of the row a finding is: its heading, and its value, typed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FindingColumn {
    /// The column's heading, as the query named it.
    pub name: ColumnName,
    /// Its value, typed; nothing for a value that reads as nothing.
    pub value: Slot,
}

/// A line of a file, counting from one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Line(pub u32);

/// A character of a line, counting from one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Column(pub u32);

/// Where in its file a finding is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Place {
    /// The finding has no place in the file: it is of the file, or of several.
    Missing,
    /// The finding starts here.
    At {
        /// The line.
        line: Line,
        /// The character in the line.
        column: Column,
    },
}

/// Something a rule could not decide: a relation the rule reads is read only in part in a file, so what it found there may be
/// missing findings, and what it would have found from the absence of rows is not answered.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Unknown {
    /// The rule that could not decide.
    pub rule: RuleId,
    /// The file in which a relation is read only in part.
    pub path: RelativePath,
    /// What is partial there and why.
    pub message: Message,
}

/// The part of the system a failure comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Port {
    /// Listing the repository's files.
    Corpus,
    /// What the run keeps for the next.
    Store,
    /// The store of facts.
    Facts,
    /// The languages.
    Language,
    /// Reading the machine's memory.
    Memory,
    /// Starting the workers.
    Scheduler,
    /// Keeping the findings.
    Output,
}

/// What a lint answers.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct LintReport {
    /// Required project lookups, reported also when the project was reused from cache.
    pub dependencies: Vec<ProjectDependency>,
    /// Every violation found.
    pub findings: Vec<Finding>,
    /// The files no linked language read, with why.
    pub unread: Vec<Unread>,
    /// What the rules could not decide.
    pub unknowns: Vec<Unknown>,
    /// Rules explicitly skipped because a file's syntax was rejected.
    pub skipped: Vec<Skipped>,
}

/// An opaque project dependency content fingerprint. Use `dependency_fingerprint` to compare bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DependencyFingerprint(pub [u8; 16]);

/// One file the project examined, whether present or absent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectDependency {
    /// Path from the run root, possibly containing `../`.
    pub path: RelativePath,
    /// The observed bytes' fingerprint, or an explicitly missing file.
    pub state: ProjectDependencyState,
}

/// State of a required project lookup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectDependencyState {
    /// Content fingerprint supplied by the engine's digest adapter.
    Present(DependencyFingerprint),
    /// The path was examined and no file was present.
    Missing,
}

/// A rule the engine did not run for a file because its applicability condition was not met.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Skipped {
    /// The rule.
    pub rule: RuleId,
    /// The file.
    pub path: RelativePath,
    /// Why the rule was skipped.
    pub reason: ryo_values::domain::runs::types::SkipReason,
}

/// A relation the host declares beside ryo's own: the name a rule writes, the columns its rows have, and the rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostRelation {
    /// The name a rule writes, as the host chose it.
    pub name: HostRelationName,
    /// The columns, in order.
    pub columns: Vec<HostColumn>,
    /// The rows, each with a cell of the kind of every column.
    pub rows: Vec<HostRow>,
}

/// One row of a relation the host declares: a cell for every column, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostRow(pub Vec<Cell>);

/// The name of a parameter of a rule's query.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParameterName(pub Box<str>);

/// The text a parameter is bound to, or one member of the set it is bound to.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParameterText(pub Box<str>);

/// A whole number a parameter is bound to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParameterNumber(pub i64);

/// A list or a set of whole numbers a parameter is bound to.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParameterNumbers(pub Vec<i64>);

/// A yes or no a parameter is bound to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FlagState {
    /// The flag is raised.
    On,
    /// The flag is not raised.
    Off,
}

/// What a parameter of a rule is bound to.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParameterValue {
    /// A whole number.
    Number(ParameterNumber),
    /// A yes or no.
    Flag(FlagState),
    /// Text.
    Text(ParameterText),
    /// A list or a set of numbers.
    Numbers(ParameterNumbers),
    /// A list or a set of text.
    Texts(Vec<ParameterText>),
    /// A list of sets of text: the sets in order.
    TextSets(Vec<Vec<ParameterText>>),
    /// A list of tables of text, each a row: what a `list<table>` parameter is bound to.
    Tables(Vec<ParameterTable>),
}

/// One table of a `list<table>` parameter: a row, its cells by key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParameterTable(pub Vec<ParameterCell>);

/// One cell of a table of a `list<table>` parameter: the key it is under and its text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParameterCell {
    /// The key.
    pub key: ParameterText,
    /// The text.
    pub value: ParameterText,
}
