//! Exact lookup and shared ownership of a project observation batch.
use super::{entries, observed, receipt, state};
use crate::domain::{
    files::types::FilePath,
    project_context::types::{EntryObservation, EntryState},
};
use std::sync::Arc;

fn entry(path: &str, state: EntryState) -> EntryObservation {
    EntryObservation {
        path: FilePath(path.into()),
        state,
    }
}

#[test]
fn sorted_lookup_shares_one_batch_and_refuses_duplicate_or_unknown_paths() {
    let batch = observed(vec![
        entry("b.py", EntryState::File),
        entry("a", EntryState::Directory),
        entry("c", EntryState::Unknown),
        entry("a", EntryState::File),
    ]);
    let shared = batch.clone();
    assert!(Arc::ptr_eq(&batch.0, &shared.0));
    assert_eq!(
        entries(&batch)
            .iter()
            .map(|each| each.path.0.as_ref())
            .collect::<Vec<_>>(),
        ["a", "a", "b.py", "c"]
    );
    assert_eq!(receipt(&batch), receipt(&shared));
    assert_eq!(state(&batch, &FilePath("a".into())), None);
    assert_eq!(
        state(&batch, &FilePath("b.py".into())),
        Some(EntryState::File)
    );
    assert_eq!(state(&batch, &FilePath("c".into())), None);
    assert_eq!(state(&batch, &FilePath("missing".into())), None);
}
