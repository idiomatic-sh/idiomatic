//! The types of the `reaching` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use tastemaker_values::domain::paths::types::RepoPath;
use tastemaker_values::domain::placement::errors::PlacementError;
use tastemaker_values::domain::scopes::types::ScopeIndex;
use tastemaker_values::domain::selectors::types::{
    GlobIndex, Resolved, ResolvedSelectors, Selectors,
};

/// A rule entry's `include` and `exclude`: what decides the files the entry reaches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SelectorPair<'p> {
    /// Its `include`.
    pub include: &'p Selectors,
    /// Its `exclude`.
    pub exclude: &'p Selectors,
}

/// A scope and one entry's `include` and `exclude` there: what the files an entry reaches are kept by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Selection<'p> {
    /// The scope.
    pub scope: ScopeIndex,
    /// The entry's selectors.
    pub pair: SelectorPair<'p>,
}

/// What a scope's set of globs makes of one item of a selector list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mark {
    /// A glob at the scope that decides alone: its place in the scope's set.
    Glob(GlobIndex),
    /// Everything beneath the scope: the pattern `.`.
    Whole,
    /// Anything else, tested alone.
    Alone,
}

/// One item of a selector list, as the scope's set reads it.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Item<'r> {
    /// The glob at this place of the scope's set.
    Glob(GlobIndex),
    /// Everything beneath the scope: the pattern `.`.
    Whole,
    /// Anything else, tested alone.
    Alone(&'r Resolved),
}

/// What one scope answers for one entry's `include` and `exclude`: the files they reach, or why they could not.
#[derive(Clone, Debug)]
pub(crate) struct ScopeReach<'w> {
    /// The entry's selectors.
    pub pair: SelectorPair<'w>,
    /// The files, or why not.
    pub files: Result<Vec<RepoPath>, PlacementError>,
}

/// One entry of a scope: its selectors, and its `include` and `exclude` as `Lists` reads them.
#[derive(Clone, Debug)]
pub(crate) struct EntryLists<'w, Lists> {
    /// The entry's selectors.
    pub pair: SelectorPair<'w>,
    /// Its `include`.
    pub include: Lists,
    /// Its `exclude`.
    pub exclude: Lists,
}

/// A scope's entries' selector lists that resolved, and the answers of those that did not.
#[derive(Debug)]
pub(crate) struct ScopeLists<'w> {
    /// The lists that resolved.
    pub lists: Vec<EntryLists<'w, ResolvedSelectors>>,
    /// The entries that did not, each with why.
    pub answers: Vec<ScopeReach<'w>>,
}

/// How many globs a scope's set holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SetWidth(pub usize);
