//! Accepted state follows the injected operations, folded afresh each time, refuses invalid graphs, and asks git for
//! renames only when a record's file is gone from the working tree.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::errors::AcceptedStateError;
use super::service::AcceptedState;
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use crate::domain::operations::types::{
    Count, Day, Operation, OperationId, Provenance, Reason, Record,
};
use crate::domain::violations::types::{
    Fingerprint, Key, Level, MessageText, RepoPath, RuleId, Standing, ToolId,
};
use crate::outbound::lock::fixtures::fake::LockFake;
use crate::outbound::lock::interface::LockPort as _;
use crate::outbound::repository::fixtures::fake::RepositoryFake;

#[derive(Clone, Copy, Debug)]
enum Scene {
    Empty,
    Changed,
    Invalid,
    /// A record accepted at `src/a.rs`, which the working tree holds.
    Unmoved,
    /// A record accepted at `src/a.rs`, which the working tree no longer holds.
    Gone,
}
type Input = Scene;
/// The heads of the fold before the change and after it, and how often git's rename detection was asked.
type Value = std::borrow::Cow<'static, str>;
type Failure = AcceptedStateError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_empty_injected_store_has_no_heads"),
        word: Word::Succeeds,
        input: || Scene::Empty,
        expect: Expect::Value(std::borrow::Cow::Borrowed("heads 0 then 0")),
    },
    Case {
        name: CaseName("a_changed_injected_store_is_folded_afresh"),
        word: Word::Succeeds,
        input: || Scene::Changed,
        expect: Expect::Value(std::borrow::Cow::Borrowed("heads 0 then 1")),
    },
    Case {
        name: CaseName("records_whose_files_are_in_the_working_tree_ask_git_for_no_renames"),
        word: Word::Succeeds,
        input: || Scene::Unmoved,
        expect: Expect::Value(std::borrow::Cow::Borrowed("1 record, renames asked 0")),
    },
    Case {
        name: CaseName("a_record_whose_file_is_gone_asks_git_for_its_renames"),
        word: Word::Succeeds,
        input: || Scene::Gone,
        expect: Expect::Value(std::borrow::Cow::Borrowed("1 record, renames asked 1")),
    },
    Case {
        name: CaseName("an_invalid_graph_is_refused"),
        word: Word::Fails,
        input: || Scene::Invalid,
        expect: Expect::Refusal(|error| {
            if matches!(error, AcceptedStateError::Fold(_)) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
];

/// An operation that accepts `accepted`, with `parents`.
fn operation(parents: Vec<OperationId>, accepted: Vec<Record>) -> Operation {
    Operation {
        parents,
        covers: Vec::new(),
        accepted,
        removed: Vec::new(),
        perf: Vec::new(),
        perf_removed: Vec::new(),
    }
}

/// A record accepted at `src/a.rs`, its id derived by `lock`.
fn record(lock: &LockFake) -> Record {
    let key = Key {
        tool: ToolId("lint".into()),
        rule: RuleId("r".into()),
        path: RepoPath("src/a.rs".into()),
        level: Level::Warning,
        standing: Standing::Reported,
        fingerprint: Fingerprint(1),
    };
    Record {
        id: lock.id(&key),
        key,
        count: Count(1),
        message: MessageText("m".into()),
        since_rev: Provenance::Unknown,
        since: Day("2026-10-28".into()),
        reasons: vec![Reason("why".into())],
    }
}

fn subject(scene: Input) -> Result<Value, Failure> {
    let lock = Arc::new(LockFake::default());
    if let Scene::Unmoved | Scene::Gone = scene {
        lock.append(&operation(Vec::new(), vec![record(&lock)]))?;
        let repository = Arc::new(RepositoryFake {
            files: if matches!(scene, Scene::Unmoved) {
                vec![RepoPath("src/a.rs".into())]
            } else {
                vec![RepoPath("src/b.rs".into())]
            },
            ..RepositoryFake::default()
        });
        let service = AcceptedState::new(
            Arc::<LockFake>::clone(&lock),
            Arc::<RepositoryFake>::clone(&repository),
        );
        let records = service.current(&RepositoryFake::default().head)?;
        return Ok(std::borrow::Cow::Owned(format!(
            "{} record, renames asked {}",
            records.len(),
            repository.renamed.load(Ordering::Relaxed)
        )));
    }
    let service = AcceptedState::new(
        Arc::<LockFake>::clone(&lock),
        Arc::new(RepositoryFake::default()),
    );
    let first = service.folded()?;
    match scene {
        Scene::Empty | Scene::Unmoved | Scene::Gone => {}
        // The seeded operation's id is not needed: the service reads the lock back through the same fake.
        Scene::Changed => lock.append(&operation(Vec::new(), Vec::new()))?,
        Scene::Invalid => {
            lock.append(&operation(vec![OperationId("missing".into())], Vec::new()))?;
        }
    }
    let next = service.folded()?;
    Ok(std::borrow::Cow::Owned(format!(
        "heads {} then {}",
        first.heads.len(),
        next.heads.len()
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
