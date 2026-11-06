//! The `project_context` service's cases, on values alone.

use super::service::{entries, observed, receipt, state};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    files::types::FilePath,
    project_context::types::{EntryObservation, ObservedEntryState},
};
use std::sync::Arc;

/// A scenario: what a row asks of the service.
type Input = fn() -> bool;
/// What a scenario answers: whether everything it states holds.
type Value = bool;
type Failure = ();

fn entry(path: &str, state: ObservedEntryState) -> EntryObservation {
    EntryObservation {
        path: FilePath(path.into()),
        state,
    }
}

fn sorted_lookup_shares_one_batch_and_refuses_duplicate_or_unknown_paths() -> bool {
    let batch = observed(vec![
        entry("b.py", ObservedEntryState::File),
        entry("a", ObservedEntryState::Directory),
        entry("c", ObservedEntryState::Unknown),
        entry("a", ObservedEntryState::File),
    ]);
    let shared = batch.clone();
    Arc::ptr_eq(&batch.0, &shared.0)
        && entries(&batch)
            .iter()
            .map(|each| each.path.0.as_ref())
            .collect::<Vec<_>>()
            == ["a", "a", "b.py", "c"]
        && receipt(&batch) == receipt(&shared)
        && state(&batch, &FilePath("a".into())).is_none()
        && state(&batch, &FilePath("b.py".into())) == Some(ObservedEntryState::File)
        && state(&batch, &FilePath("c".into())).is_none()
        && state(&batch, &FilePath("missing".into())).is_none()
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("sorted_lookup_shares_one_batch_and_refuses_duplicate_or_unknown_paths"),
    word: Word::Succeeds,
    input: || sorted_lookup_shares_one_batch_and_refuses_duplicate_or_unknown_paths,
    expect: Expect::Value(true),
}];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(scenario())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
