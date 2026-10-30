//! The types of the `reaching` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::paths::types::RepoPath;
use crate::domain::placement::errors::PlacementError;
use crate::domain::scopes::types::ScopeIndex;
use crate::domain::selectors::types::{GlobIndex, Resolved, ResolvedSelectors, Selectors};

/// A rule entry's `include` and `exclude`: what decides the files the entry reaches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SelectorPair<'p> {
    /// Its `include`.
    pub include: &'p Selectors,
    /// Its `exclude`.
    pub exclude: &'p Selectors,
}

/// A scope and one entry's `include` and `exclude` there: what the files an entry reaches are kept by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Selection<'p> {
    /// The scope.
    pub scope: ScopeIndex,
    /// The entry's selectors.
    pub pair: SelectorPair<'p>,
}

/// What a scope's set of globs makes of one item of a selector list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    /// A glob at the scope that decides alone: its place in the scope's set.
    Glob(GlobIndex),
    /// Everything beneath the scope: the pattern `.`.
    Whole,
    /// Anything else, tested alone.
    Alone,
}

/// One item of a selector list, as the scope's set reads it.
#[derive(Clone, Copy, Debug)]
pub enum Item<'r> {
    /// The glob at this place of the scope's set.
    Glob(GlobIndex),
    /// Everything beneath the scope: the pattern `.`.
    Whole,
    /// Anything else, tested alone.
    Alone(&'r Resolved),
}

/// What one scope answers for one entry's `include` and `exclude`: the files they reach, or why they could not.
#[derive(Clone, Debug)]
pub struct ScopeReach<'w> {
    /// The entry's selectors.
    pub pair: SelectorPair<'w>,
    /// The files, or why not.
    pub files: Result<Vec<RepoPath>, PlacementError>,
}

/// One entry of a scope: its selectors, and its `include` and `exclude` as `Lists` reads them.
#[derive(Clone, Debug)]
pub struct EntryLists<'w, Lists> {
    /// The entry's selectors.
    pub pair: SelectorPair<'w>,
    /// Its `include`.
    pub include: Lists,
    /// Its `exclude`.
    pub exclude: Lists,
}

/// A scope's entries' selector lists that resolved, and the answers of those that did not.
#[derive(Debug)]
pub struct ScopeLists<'w> {
    /// The lists that resolved.
    pub lists: Vec<EntryLists<'w, ResolvedSelectors>>,
    /// The entries that did not, each with why.
    pub answers: Vec<ScopeReach<'w>>,
}

/// How many globs a scope's set holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetWidth(pub usize);
