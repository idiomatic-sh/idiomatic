//! Exact-path lookup in one immutable engine-observed entry batch.
use super::types::{EntryObservation, EntryState, ObservedEntries};
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
    entries.receipt()
}

impl ObservedEntries {
    /// Materialize the existing owned project answer receipt at its output boundary.
    #[must_use]
    pub fn receipt(&self) -> Vec<EntryObservation> {
        self.0.to_vec()
    }
}

/// Borrow the exact ordered observations for receipt comparison.
#[must_use]
pub fn entries(entries: &ObservedEntries) -> &[EntryObservation] {
    &entries.0
}

/// The observed entry state; duplicates or an unobserved path remain unknown.
#[must_use]
pub fn state(entries: &ObservedEntries, path: &FilePath) -> Option<EntryState> {
    let index = entries
        .0
        .binary_search_by(|entry| entry.path.cmp(path))
        .ok()?;
    if (index > 0 && entries.0[index - 1].path == *path)
        || (index + 1 < entries.0.len() && entries.0[index + 1].path == *path)
    {
        return None;
    }
    match entries.0[index].state {
        EntryState::Unknown => None,
        state => Some(state),
    }
}

#[cfg(test)]
#[path = "service_test.rs"]
mod tests;
