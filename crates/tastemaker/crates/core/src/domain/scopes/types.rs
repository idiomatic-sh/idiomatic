//! The types of the `scopes` concept: plain data, public fields and derives, with no constructor and no inherent impl.
//! A scope is where one composition applies; these are the scopes, what their entries placed and matched, and the
//! counts and pairs that wait for the conditions.

use std::collections::{BTreeMap, BTreeSet};

use crate::domain::captures::types::{CaptureName, Captures, Interpolations};
use crate::domain::entries::types::{Amount, ClosedChoice, Count, ParamName, Value};
use crate::domain::findings::types::Finding;
use crate::domain::packages::types::Release;
use crate::domain::paths::types::RepoPath;
use crate::domain::selectors::types::Selectors;

/// The place of a scope among the scopes of a placement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScopeIndex(pub usize);

/// The place of an entry among the entries of its kind in its composition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntryIndex(pub usize);

/// Which composition a scope applies.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CompositionRef {
    /// The root `tastemaker.toml`.
    Root,
    /// The local file at this place among the workspace's locals.
    Local(EntryIndex),
    /// The spec of this release.
    Spec(Release),
}

/// How a scope came to be.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// A `tastemaker.toml`, root or local.
    File,
    /// An instance of the spec entry at `entry` of the scope at `parent`.
    Instance {
        /// The scope whose composition holds the entry.
        parent: ScopeIndex,
        /// The entry, among its composition's spec entries.
        entry: EntryIndex,
    },
}

/// How output names a scope or an entry: a file's path, or `package/name(alias)`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Identity(pub Box<str>);

/// Where one composition applies: a `tastemaker.toml` at its directory, or one spec instance at its directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scope {
    /// Its directory.
    pub dir: RepoPath,
    /// How findings name it.
    pub identity: Identity,
    /// Where a finding about the scope itself is reported.
    pub anchor: RepoPath,
    /// Whether only placed files may exist.
    pub closed: ClosedChoice,
    /// The spec's params as text, interpolated into its entries' paths.
    pub values: Interpolations,
    /// The same params with their types, as a rule entry passes them on.
    pub typed: BTreeMap<ParamName, Value>,
    /// What the spec entry's `include` captured for this instance, with the enclosing instance's.
    pub captures: Captures,
    /// The composition it applies.
    pub composition: CompositionRef,
    /// The paths, from `dir`, its own rules do not read: its spec's `rules-exclude`.
    pub rules_exclude: Selectors,
    /// The directories of the instances nested in it, where it places no file; its rules still read them.
    pub nested: Vec<RepoPath>,
    /// How it came to be.
    pub origin: Origin,
}

/// A file a template entry placed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacedFile {
    /// The scope whose entry placed it.
    pub scope: ScopeIndex,
    /// The entry, among its composition's template entries.
    pub entry: EntryIndex,
    /// What the entry's `file` captured.
    pub captures: Captures,
}

/// What one template entry matched in one scope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Matched {
    /// The scope.
    pub scope: ScopeIndex,
    /// The entry.
    pub entry: EntryIndex,
    /// The names its `file` captures.
    pub names: BTreeSet<CaptureName>,
    /// Each distinct capture tuple its files took.
    pub tuples: BTreeSet<Captures>,
    /// The files, or the captured values when its `file` captures.
    pub counted: Amount,
    /// The names captured before the last: when there are any, its count holds per value.
    pub leading: BTreeSet<CaptureName>,
}

/// A spec entry whose `include` captures names: what its instances captured, paired with the template entries of the
/// root file that capture the same names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapturingEntry {
    /// The entry's identity.
    pub identity: Identity,
    /// Its first `include`, as written.
    pub include: IncludeSpelling,
    /// The fewest its count accepts when that does not depend on a condition.
    pub minimum: Amount,
    /// The names its `include` captures.
    pub names: BTreeSet<CaptureName>,
    /// Each distinct capture tuple its instances took.
    pub tuples: BTreeSet<Captures>,
    /// The scope whose composition holds the entry.
    pub scope: ScopeIndex,
}

/// A count to check once the conditions are answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CountCheck {
    /// The check: the entry's `count`.
    pub rule: crate::domain::findings::types::FindingRule,
    /// Where it is reported.
    pub path: RepoPath,
    /// The scope whose conditions the count reads.
    pub scope: ScopeIndex,
    /// The count.
    pub count: Count,
    /// How many were found.
    pub found: Amount,
    /// What the finding says.
    pub message: CountMessage,
}

/// What a count's finding says, its count written once the condition is answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CountMessage {
    /// A spec entry's directories.
    Spec {
        /// The entry.
        identity: Identity,
    },
    /// A template entry's files or captured values in a scope.
    Template {
        /// The entry.
        identity: Identity,
        /// Its `file`.
        file: FileSpelling,
        /// What it counts.
        unit: CountUnit,
        /// The scope's directory.
        directory: RepoPath,
    },
    /// One group of an entry whose pattern captures more than one name.
    Grouped {
        /// The entry.
        identity: Identity,
        /// The group's captures, as a message writes them.
        captures: CapturesSpelling,
    },
}

/// What a template entry's count counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CountUnit {
    /// Files.
    Files,
    /// Captured values.
    Values,
}

/// Findings that hold only when an entry's count, its condition answered, requires one or more.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairCheck {
    /// The scope whose conditions the count reads.
    pub scope: ScopeIndex,
    /// The count.
    pub count: Count,
    /// The findings.
    pub findings: Vec<Finding>,
}

/// A template entry's match while placement runs: with its compiled `file` and `exclude`, for the counts of its groups.
#[derive(Clone)]
pub(crate) struct MatchedWorking {
    /// What it matched.
    pub matched: Matched,
    /// Its `file`, placed in the scope.
    pub matcher: crate::domain::selectors::types::PathMatcher,
    /// Its `exclude`, placed in the scope.
    pub exclude: crate::domain::selectors::types::ResolvedSelectors,
}

/// One `include` pattern of a grouped spec entry: its leading names and what its instances captured.
#[derive(Clone)]
pub(crate) struct PatternWorking {
    /// The compiled pattern.
    pub matcher: crate::domain::selectors::types::PathMatcher,
    /// The names before its last capture.
    pub leading: BTreeSet<CaptureName>,
    /// What its instances captured.
    pub tuples: BTreeSet<Captures>,
}

/// A spec entry whose `include` captures, while placement runs.
#[derive(Clone)]
pub(crate) struct AppliedWorking {
    /// What it captured.
    pub applied: CapturingEntry,
    /// Its count.
    pub count: Count,
    /// Whether it counts per group of leading captures.
    pub grouped: Grouping,
    /// Each `include` pattern.
    pub patterns: Vec<PatternWorking>,
    /// Its `exclude`, placed.
    pub exclude: crate::domain::selectors::types::ResolvedSelectors,
}

/// Whether a spec entry's count holds per group of its leading captures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Grouping {
    /// Across the scope.
    Whole,
    /// Per group.
    Grouped,
}

/// One spec entry applied: what its count counts, checked once the conditions are answered.
#[derive(Clone)]
pub(crate) struct CountedEntry {
    /// The scope whose composition holds the entry.
    pub parent: ScopeIndex,
    /// The entry's identity.
    pub identity: Identity,
    /// Its count.
    pub count: Count,
    /// The directories, or the capture tuples, it applied at.
    pub counted: Amount,
    /// Whether it counts per group.
    pub grouped: Grouping,
}

/// What a condition answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    /// Its rule selected a row.
    Holds,
    /// Its rule selected none.
    Fails,
    /// Its rule could not run: the counts that read it are not judged.
    Unanswered,
}

/// Whether a directory holds a file directly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileHolding {
    /// A file lies directly in it.
    HoldsFiles,
    /// None does: it holds only directories, or nothing.
    HoldsNone,
}

/// Whether a composition places files through templates, its own or those of the specs it places.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilePlacing {
    /// It places files.
    PlacesFiles,
    /// It places none.
    PlacesNone,
}

/// Whether a composition closes its directory: it is closed and places files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Closing {
    /// Only placed files may exist in its directory.
    Closes,
    /// Any file may.
    Open,
}

/// The name of the spec an instance applies.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpecLabel(pub String);

/// The name other entries refer to an instance's spec entry by.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EntryLabel(pub String);

/// The spec's name and the spec entry's reference of an instance: empty for a file's own scope.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Labels {
    /// The spec's name.
    pub spec: SpecLabel,
    /// The spec entry's reference.
    pub entry: EntryLabel,
}

/// What a check of the scopes found, and every check of it that ran.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScopeCheck {
    /// The findings.
    pub findings: Vec<Finding>,
    /// Every check that ran.
    pub evaluated: BTreeSet<crate::domain::findings::types::FindingRule>,
}

/// A spec entry's first `include`, as written, for a message.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IncludeSpelling(pub Box<str>);

/// A template entry's `file`, as written, for a message.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileSpelling(pub Box<str>);

/// The captures of a group, as a message writes them.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapturesSpelling(pub Box<str>);

impl std::fmt::Display for IncludeSpelling {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::fmt::Display for FileSpelling {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::fmt::Display for CapturesSpelling {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A condition of one scope: what an answer to it is kept by.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConditionAt {
    /// The scope whose composition declares the condition.
    pub scope: ScopeIndex,
    /// The condition.
    pub condition: crate::domain::entries::types::ConditionName,
}
