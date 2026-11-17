//! The types of the `placement` concept: plain data, public fields and derives, with no constructor and no inherent impl.
//! Placement turns every entry of a workspace into scopes, instances and placed files, and says what it found.

use std::collections::{BTreeMap, BTreeSet};

use crate::domain::captures::types::Captures;
use crate::domain::findings::types::Finding;
use crate::domain::paths::types::RepoPath;
use crate::domain::scopes::types::{
    AppliedWorking, CapturingEntry, CountCheck, CountedEntry, Matched, MatchedWorking, PairCheck,
    PlacedFile, Scope,
};
use crate::domain::selectors::types::{ResolvedSelectors, SelectorIndex};

/// Everything placement worked out: the scopes, the placed files, what each entry matched, and what it found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Placement {
    /// The scopes, the files' first.
    pub scopes: Vec<Scope>,
    /// The placed files.
    pub placed: BTreeMap<RepoPath, PlacedFile>,
    /// What each template entry matched.
    pub matched: Vec<Matched>,
    /// The spec entries whose `include` captures.
    pub applied: Vec<CapturingEntry>,
    /// The counts to check once the conditions are answered.
    pub counts: Vec<CountCheck>,
    /// The pairs to report once the conditions are answered.
    pub pairs: Vec<PairCheck>,
    /// What placement found.
    pub findings: Vec<Finding>,
    /// Every check that ran.
    pub evaluated: BTreeSet<crate::domain::findings::types::FindingRule>,
    /// The files under a `rules-exclude`: only placement and `closed` read them.
    pub excluded: BTreeSet<RepoPath>,
    /// What the closed directories hold, for the closed query to decide the strays from.
    pub closings: crate::domain::closing::types::Closings,
}

/// What applying a spec entry reads besides what the placement built: the workspace, how a selector compiles and how a
/// param's value is written.
#[derive(Clone, Copy)]
pub(crate) struct Applying<'a> {
    /// The workspace and its definitions.
    pub workspace: &'a crate::domain::definitions::types::LoadedWorkspace,
    /// How a selector's pattern compiles.
    pub compile: &'a dyn crate::domain::selectors::types::PatternCompiler,
    /// How a param's value is written as a path or a template reads it.
    pub written: &'a dyn crate::domain::entries::types::ValueWriter,
}

/// Everything a placement builds as it runs.
#[derive(Clone, Default)]
pub(crate) struct Working {
    /// What it worked out so far.
    pub placement: Placement,
    /// What each template entry matched, with its matcher.
    pub matched: Vec<MatchedWorking>,
    /// The spec entries whose `include` captures, with what their counts are judged from.
    pub applied: Vec<AppliedWorking>,
    /// Every spec entry applied, with how many directories it counted.
    pub applications: Vec<CountedEntry>,
}

/// A directory a spec entry applies to: what its `include` captured and which of its patterns matched it first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectedDirectory {
    /// The directory.
    pub directory: RepoPath,
    /// What the pattern captured.
    pub captures: Captures,
    /// The pattern, by its place in `include`.
    pub pattern: SelectorIndex,
}

/// What a spec entry selects: its `include` and `exclude` resolved, and each directory it applies to.
#[derive(Clone, Debug)]
pub struct Selected {
    /// Its `include`, resolved.
    pub include: ResolvedSelectors,
    /// Its `exclude`, resolved.
    pub exclude: ResolvedSelectors,
    /// Each directory it applies to.
    pub found: Vec<SelectedDirectory>,
}

/// Whether a spec is already applied at a directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Duplication {
    /// It is: a `#coverage` error, and the directory is not placed again.
    Twice,
    /// It is not.
    Once,
}
