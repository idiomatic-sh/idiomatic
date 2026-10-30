//! What the `open` use case promises, against the fakes of the ports it calls.

use std::sync::Arc;

use super::errors::OpenError;
use super::interface::OpenUseCase as _;
use super::types::OpenRequest;
use super::use_case::OpenUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::outbound::language::fixtures::fake::{FakeCatalog, LanguageFake};
use crate::outbound::memory::fixtures::fake::MemoryFake;
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_values::domain::runs::types::Budget;
use ryo_values::domain::runs::types::MemoryBytes;

/// What a row gives the use case: what the catalog answers, whether memory cannot be read, and the budget asked for.
type Input = (FakeCatalog, bool, u64);
/// Whether the session is over the root asked.
type Value = usize;
type Failure = OpenError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("opens_a_session_over_a_root"),
        word: Word::Succeeds,
        input: || (FakeCatalog::Lists, false, 1024),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_budget_the_machine_cannot_give_is_refused"),
        word: Word::Refuses,
        input: || (FakeCatalog::Lists, false, u64::MAX),
        expect: Expect::Refusal(|error| selects(matches!(error, OpenError::Memory { .. }))),
    },
    Case {
        name: CaseName("fails_when_language_port_fails"),
        word: Word::Fails,
        input: || (FakeCatalog::Fails, false, 1024),
        expect: Expect::Refusal(|error| selects(matches!(error, OpenError::Language { .. }))),
    },
    Case {
        name: CaseName("fails_when_memory_cannot_be_read"),
        word: Word::Fails,
        input: || (FakeCatalog::Lists, true, 1024),
        expect: Expect::Refusal(|error| selects(matches!(error, OpenError::Memory { .. }))),
    },
];

fn subject((catalog, memory_fails, budget): Input) -> Result<Value, Failure> {
    let use_case = OpenUseCaseImpl::new(
        Arc::new(LanguageFake {
            catalog,
            ..LanguageFake::default()
        }),
        Arc::new(MemoryFake {
            fails: memory_fails,
            asked: Budget::Bytes(MemoryBytes(budget)),
            ..MemoryFake::default()
        }),
    );
    use_case
        .open(OpenRequest {
            repository: ryo_language::inbound::use_cases::project::types::Repository::Disk,
            root: RootPath(".".into()),
        })
        .map(|report| usize::from(report.session.root == RootPath(".".into())))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
