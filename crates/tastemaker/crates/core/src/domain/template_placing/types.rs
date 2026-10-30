//! The types of the `template_placing` concept: plain data, public fields and derives, with no constructor and no
//! inherent impl.

use std::collections::BTreeSet;

use crate::domain::captures::types::Captures;
use crate::domain::entries::types::EntryIdentity;
use crate::domain::findings::types::Finding;
use crate::domain::paths::types::RepoPath;
use crate::domain::scopes::types::MatchedWorking;
use crate::domain::selectors::types::{PartInstance, PathMatcher, PlacedPath, ResolvedSelectors};

/// Whether a selector list names instances or placed files, which are then gathered for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartNaming {
    /// It names a part or a placed file.
    NamesParts,
    /// It names neither.
    NamesNone,
}

/// The instances and placed files a selector of a scope can name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Known {
    /// Every spec instance.
    pub instances: Vec<PartInstance>,
    /// Every placed file, with the entry that placed it.
    pub placed: Vec<PlacedPath>,
}

/// A file a template entry matched, with what its pattern captured.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchedFile {
    /// The file.
    pub file: RepoPath,
    /// What the entry's pattern captured of its path.
    pub captures: Captures,
}

/// What one template entry of one scope matched: the files it places with what they captured, its pattern and its
/// `exclude` resolved.
#[derive(Clone)]
pub(crate) struct TemplateMatch {
    /// The entry's identity.
    pub identity: EntryIdentity,
    /// The files, with their captures.
    pub files: Vec<MatchedFile>,
    /// The entry's file pattern placed at the scope.
    pub matcher: PathMatcher,
    /// Its `exclude` resolved.
    pub exclude: ResolvedSelectors,
}

/// What placing the template entries found besides the files they place: findings, the checks that ran, and what each
/// entry matched.
#[derive(Clone, Default)]
pub(crate) struct TemplatesPlaced {
    /// The findings.
    pub findings: Vec<Finding>,
    /// Every check that ran.
    pub evaluated: BTreeSet<crate::domain::findings::types::FindingRule>,
    /// What each template entry matched, in order.
    pub matched: Vec<MatchedWorking>,
}
