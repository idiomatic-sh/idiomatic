//! The types of the `applications` concept: plain data, public fields and derives, with no constructor and no inherent
//! impl. An application is a rule placed somewhere; the plan is every application as the check's lint calls take it, and
//! what each answer becomes.

use std::collections::{BTreeMap, BTreeSet};

use crate::domain::definitions::types::{ColumnName, Diagnostic, QueryText};
use crate::domain::entries::types::{ConditionName, ParamName, RuleEntry, Value};
use crate::domain::findings::types::WorkspaceName;
use crate::domain::findings::types::{Finding, MessageText, RuleId};
use crate::domain::packages::types::{DefinitionName, PackageName, Release, VersionText};
use crate::domain::paths::types::RepoPath;
use crate::domain::scopes::types::{CountCheck, Identity, PairCheck, ScopeIndex};
use crate::domain::selectors::types::{PartInstance, PatternCompiler, PlacedPath, SetCompiler};

/// The id a rule of the lint call carries: `rule-<n>`, `condition-<n>`, `tastemaker-comments`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LintRuleId(pub Box<str>);

/// A glob over the paths of the analysis root, as the lint call takes it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GlobText(pub Box<str>);

/// One rule of the lint call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedRule {
    /// Its id.
    pub id: LintRuleId,
    /// Its query.
    pub query: QueryText,
    /// What its params are bound to.
    pub parameters: BTreeMap<ParamName, Value>,
    /// The files it reads, relative to the analysis root.
    pub include: Vec<GlobText>,
    /// The files of `include` it does not read: the nested instances and the `exclude` of the entries it carries.
    pub exclude: Vec<GlobText>,
}

/// What a rule's finding is rendered with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rendering {
    /// The id its findings carry: the release's.
    pub rule: RuleId,
    /// Its diagnostic.
    pub diagnostic: Diagnostic,
}

/// What a rule of the lint call is for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Purpose {
    /// A placed rule: its findings are reported.
    Rule(Rendering),
    /// A condition of a scope: yes when it selects a row.
    Condition {
        /// The scope.
        scope: ScopeIndex,
        /// The condition.
        name: ConditionName,
        /// The entry that answers it.
        identity: Identity,
    },
    /// The comments of the files that may hold an ignore.
    Comments,
    /// A shared relation of the placed files and of their templates' rendered contents, which the contracts compare.
    Declarations(crate::domain::contracts::types::Syntax),
    /// The imports the entries' `imports` hold, read from one relation.
    Imports(crate::domain::imports::types::Relation),
}

/// One row of `tastemaker.placed`: a file a template entry placed, and the entry.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlacedRow {
    /// The file, relative to the analysis root.
    pub path: RepoPath,
    /// The entry's reference.
    pub entry: DefinitionName,
    /// The template's package.
    pub package: PackageName,
    /// The template's name.
    pub template: DefinitionName,
    /// The template's version.
    pub version: VersionText,
    /// The spec of the instance whose entry placed it, or the `tastemaker.toml`'s own entry.
    pub spec: PlacingSpec,
}

/// What placed a file of `tastemaker.placed`: an entry of the `tastemaker.toml` itself, which is of no spec, or an entry of a
/// spec's instance.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlacingSpec {
    /// An entry of the `tastemaker.toml` itself.
    Workspace,
    /// An entry of an instance of this spec.
    Instance(DefinitionName),
}

/// Everything a check sends in its lint calls and what it does with the answer: the rules, the placed rows, what
/// placement found, and the counts that wait for the conditions.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Plan {
    /// The workspace's name.
    pub workspace: WorkspaceName,
    /// The directory the lint call is rooted at, relative to the repository.
    pub base: RepoPath,
    /// The rules of the lint call.
    pub rules: Vec<PlannedRule>,
    /// What each is for.
    pub purposes: BTreeMap<LintRuleId, Purpose>,
    /// The rows of `tastemaker.placed`.
    pub placed: Vec<PlacedRow>,
    /// What placement found.
    pub findings: Vec<Finding>,
    /// The counts that wait for the conditions.
    pub counts: Vec<CountCheck>,
    /// The pairs that wait for the conditions.
    pub pairs: Vec<PairCheck>,
    /// Every rule and check that ran.
    pub evaluated: BTreeSet<crate::domain::findings::types::FindingRule>,
    /// The ids of every rule the workspace loads: what an ignore may name.
    pub known: BTreeSet<RuleId>,
    /// The files under a `rules-exclude`, whose ignores answer to no rule.
    pub excluded: BTreeSet<RepoPath>,
    /// What the closed directories hold, for the closed query to decide the strays from.
    pub closings: crate::domain::closing::types::Closings,
    /// The files the workspace governs.
    pub files: Vec<RepoPath>,
    /// Where each scope's own findings are anchored, for an answer about a scope.
    pub anchors: Vec<RepoPath>,
    /// The placed files compared with their templates' rendered contents.
    pub contracts: Vec<crate::domain::contracts::types::Contract>,
    /// What each scope's entries' `imports` hold.
    pub imports: Vec<crate::domain::imports::types::ScopeImports>,
}

/// Where a rule's finding is in its file, before the file's text says which lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoundAt {
    /// About the whole file.
    Whole,
    /// At this byte range.
    Span(crate::domain::comments::types::ByteSpan),
}

/// A finding of the lint call, rendered, its region still in bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answered {
    /// The finding, its place still whole.
    pub finding: Finding,
    /// Where in its file.
    pub located: FoundAt,
}

/// One row the lint call answered: the rule of the call, the file (relative to the repository) and its key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LintAnswer {
    /// The rule of the call.
    pub rule: LintRuleId,
    /// The file, relative to the repository.
    pub path: RepoPath,
    /// Every cell of the row, by its column's heading, typed.
    pub cells: Vec<AnswerCell>,
}

/// One cell of a row the lint call answered: its column's heading and its value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnswerCell {
    /// Its column's heading.
    pub name: ColumnName,
    /// Its value.
    pub value: CellValue,
}

/// The value of a cell of a row the lint call answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CellValue {
    /// The file the row is of, relative to the repository.
    File(RepoPath),
    /// A place in the file, by its byte range.
    Span(crate::domain::comments::types::ByteSpan),
    /// Any other value, as text.
    Text(CellText),
}

/// One thing a rule of the lint call could not decide.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LintUnknown {
    /// The rule of the call.
    pub rule: LintRuleId,
    /// The file, relative to the repository.
    pub path: RepoPath,
    /// What is partial and why.
    pub message: MessageText,
}

/// The rows the contracts compare, by file, and the files they cannot compare.
#[derive(Clone, Copy, Debug)]
pub struct Compared<'a> {
    /// The rows of the shared relations, of the placed files and their rendered contents.
    pub data: &'a BTreeMap<RepoPath, Vec<crate::domain::contracts::types::DataRow>>,
    /// The files no linked language read.
    pub unread: &'a BTreeSet<RepoPath>,
    /// The files a row of the comparison was dropped from, its node missing.
    pub dropped: &'a BTreeSet<RepoPath>,
}

/// What the lint call's answer comes to: the findings, those still to place by their file's text, the comments of the
/// files that may hold an ignore, and what each condition answered.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Judged {
    /// The findings placed.
    pub findings: Vec<Finding>,
    /// The findings whose byte range the file's text reads into lines.
    pub spans: Vec<SpannedFinding>,
    /// Each file's comments, when one of them holds `ignore:`.
    pub comments: BTreeMap<RepoPath, Vec<crate::domain::comments::types::Comment>>,
    /// The rows the contracts compare, by file: the placed files' and their rendered contents'.
    pub data: BTreeMap<RepoPath, Vec<crate::domain::contracts::types::DataRow>>,
    /// The files a row of the comparison was dropped from, its node missing.
    pub dropped: BTreeSet<RepoPath>,
    /// The files no linked language read.
    pub unread: BTreeSet<RepoPath>,
    /// The rows the imports are read from.
    pub imports: crate::domain::imports::types::ImportRows,
}

/// The value of a column of a row the lint call answered, as text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellText(pub Box<str>);

/// A finding whose byte range its file's text reads into lines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpannedFinding {
    /// The finding, its place still whole.
    pub finding: Finding,
    /// Where in its file.
    pub span: crate::domain::comments::types::ByteSpan,
}

/// The values a rule entry passes, by the param each binds.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParamSet(pub BTreeMap<ParamName, Value>);

/// One rule entry of one scope, its params resolved: what the lint call carries.
#[derive(Clone, Debug)]
pub struct Carried<'w> {
    /// The scope.
    pub scope: ScopeIndex,
    /// The entry.
    pub entry: &'w RuleEntry,
    /// The release it names.
    pub release: Release,
    /// Its params.
    pub params: ParamSet,
}

/// The instances and placed files a rule entry's selectors can name.
#[derive(Clone, Debug, Default)]
pub struct Nameable {
    /// The spec instances.
    pub instances: Vec<PartInstance>,
    /// The placed files.
    pub placed: Vec<PlacedPath>,
}

/// The matching port's compiles, as the domain is handed them: one pattern, and one set of globs.
#[derive(Clone, Copy)]
pub struct Compilers<'a> {
    /// Compiles one pattern.
    pub compile: &'a dyn PatternCompiler,
    /// Compiles a set of globs.
    pub sets: &'a dyn SetCompiler,
}

impl std::fmt::Debug for Compilers<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

/// One rule of the lint call for a release and its params: the files it reads as globs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandedRule {
    /// The release.
    pub release: Release,
    /// Its params.
    pub params: ParamSet,
    /// The files it reads.
    pub include: Vec<GlobText>,
    /// The files of `include` it does not read.
    pub exclude: Vec<GlobText>,
}
