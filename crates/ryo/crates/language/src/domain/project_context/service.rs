//! Exact-path lookup in one immutable engine-observed entry batch.
use super::types::{EntryObservation, ObservedEntries, ObservedEntryState, ProjectSearchScope};
use crate::domain::files::types::FilePath;

/// Sort an observation batch once; retain duplicates as uncertainty rather than choosing a winner.
#[must_use]
pub fn observed(mut entries: Vec<EntryObservation>) -> ObservedEntries {
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    ObservedEntries(entries.into())
}

/// Recover the exact ordered receipt without rebuilding the observation index.
#[must_use]
pub fn receipt(entries: &ObservedEntries) -> Vec<EntryObservation> {
    entries.0.to_vec()
}

/// Borrow the exact ordered observations for receipt comparison.
#[must_use]
pub fn entries(entries: &ObservedEntries) -> &[EntryObservation] {
    &entries.0
}

/// The observed entry state; duplicates or an unobserved path remain unknown.
#[must_use]
pub fn state(entries: &ObservedEntries, path: &FilePath) -> Option<ObservedEntryState> {
    let Ok(index) = entries.0.binary_search_by(|entry| entry.path.cmp(path)) else {
        return None;
    };
    if (index > 0 && entries.0[index - 1].path == *path)
        || (index + 1 < entries.0.len() && entries.0[index + 1].path == *path)
    {
        return None;
    }
    match entries.0[index].state {
        ObservedEntryState::Unknown => None,
        state @ (ObservedEntryState::File
        | ObservedEntryState::Directory
        | ObservedEntryState::Other
        | ObservedEntryState::Missing) => Some(state),
    }
}

/// Ordered search roots, or none when the host declared no search scope.
#[must_use]
pub fn search_roots(scope: &ProjectSearchScope) -> Option<&[FilePath]> {
    match scope {
        ProjectSearchScope::Undeclared => None,
        ProjectSearchScope::Declared(roots) => Some(roots),
    }
}
