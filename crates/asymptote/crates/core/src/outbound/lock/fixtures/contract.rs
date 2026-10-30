//! What every `lock` adapter must pass: an operation appended is read back under its address, appending it again
//! changes nothing, and a key's id depends on everything that identifies it.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operations::types::{
    Count, Day, Operation, OperationId, OperationText, Provenance, Reason, Record,
};
use crate::domain::violations::types::{
    Fingerprint, Key, Level, MessageText, RepoPath, RuleId, Standing, ToolId,
};
use crate::outbound::lock::errors::LockError;
use crate::outbound::lock::interface::LockPort;

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
pub enum Question {
    /// An operation appended twice is one file, read back as written.
    Appended,
    /// A suppressed key has another id than the reported one.
    Standing,
    /// Text that is not the operation its id addresses is refused.
    Misaddressed,
}

/// Which question is asked.
type Input = Question;
/// Whether the answer holds.
type Value = bool;
/// The lock's own error: every row succeeds, so one is the implementation failing the contract.
type Failure = LockError;

/// The contract's rows: every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_operation_appended_twice_is_one_file_read_back_as_written"),
        word: Word::Succeeds,
        input: || Question::Appended,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_suppressed_key_has_its_own_id"),
        word: Word::Succeeds,
        input: || Question::Standing,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("text_that_is_not_the_operation_its_id_addresses_is_refused"),
        word: Word::Succeeds,
        input: || Question::Misaddressed,
        expect: Expect::Value(true),
    },
];

/// Runs the contract's rows against one implementation, over the product it is bound to, whose lock is empty.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl LockPort) {
    run(
        CASES,
        Word::Succeeds,
        |question| -> Result<Value, Failure> {
            // A key of the contract's, at the standing a row asks for.
            let key = |standing: Standing| Key {
                tool: ToolId("cargo-clippy".into()),
                rule: RuleId("clippy::pedantic".into()),
                path: RepoPath("src/lib.rs".into()),
                level: Level::Warning,
                standing,
                fingerprint: Fingerprint(0xa823_3f95_e642_12b8),
            };
            Ok(match question {
                Question::Appended => {
                    let key = key(Standing::Reported);
                    let operation = Operation {
                        parents: Vec::new(),
                        covers: Vec::new(),
                        accepted: vec![Record {
                            id: adapter.id(&key),
                            key,
                            count: Count(2),
                            message: MessageText("say \"why\"".into()),
                            since_rev: Provenance::Unknown,
                            since: Day("2026-10-19".into()),
                            reasons: vec![Reason("a reason".into())],
                        }],
                        removed: Vec::new(),
                        perf: Vec::new(),
                        perf_removed: Vec::new(),
                    };
                    adapter.append(&operation)?;
                    adapter.append(&operation)?;
                    let read = adapter.read()?;
                    let listed = adapter.listing()?;
                    let appended = read.0.values().filter(|kept| **kept == operation).count() == 1
                        && listed.files.len() == 1;
                    // A snapshot that covers it replaces it: the lock holds the snapshot alone.
                    let snapshot = Operation {
                        covers: listed.files.clone(),
                        accepted: Vec::new(),
                        ..operation.clone()
                    };
                    adapter.compact(&snapshot)?;
                    let compacted = adapter.read()?;
                    appended
                        && compacted.0.len() == 1
                        && compacted.0.values().all(|kept| *kept == snapshot)
                }
                Question::Standing => {
                    assert!(
                        !adapter.directory().0.is_empty(),
                        "the lock names where it keeps its operations"
                    );
                    adapter.id(&key(Standing::Reported)) != adapter.id(&key(Standing::Suppressed))
                }
                Question::Misaddressed => adapter
                    .parse(
                        &OperationId("0".repeat(64).into()),
                        &OperationText("version = 1\nparents = []\n".into()),
                    )
                    .is_err(),
            })
        },
    );
}
