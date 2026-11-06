//! What the `open` use case promises, against the fakes of the ports it calls.

use std::sync::Arc;

use super::errors::OpenError;
use super::interface::OpenUseCase as _;
use super::types::OpenRequest;
use super::use_case::OpenUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run, Matched, selects};
use crate::outbound::language::fixtures::fake::{FakeCatalog, LanguageFake};
use crate::outbound::memory::fixtures::fake::MemoryFake;
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_values::domain::runs::types::Budget;
use ryo_values::domain::runs::types::MemoryBytes;

/// What a row gives the use case.
enum Ask {
    /// What the catalog answers, whether memory cannot be read, and the budget asked for.
    Root(FakeCatalog, bool, u64),
    /// A root opened over a declared scope and the inventory given for it.
    Declared,
}
/// What a row gives the use case.
type Input = Ask;
/// Whether the session is over the root asked.
type Value = usize;
type Failure = OpenError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("opens_a_session_over_a_root"),
        word: Word::Succeeds,
        input: || Ask::Root(FakeCatalog::Lists, false, 1024),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("opening_preserves_declared_project_authority"),
        word: Word::Succeeds,
        input: || Ask::Declared,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_budget_the_machine_cannot_give_is_refused"),
        word: Word::Refuses,
        input: || Ask::Root(FakeCatalog::Lists, false, u64::MAX),
        expect: Expect::Refusal(|error| selects(Matched(matches!(error, OpenError::Memory { .. })))),
    },
    Case {
        name: CaseName("fails_when_language_port_fails"),
        word: Word::Fails,
        input: || Ask::Root(FakeCatalog::Fails, false, 1024),
        expect: Expect::Refusal(|error| selects(Matched(matches!(error, OpenError::Language { .. })))),
    },
    Case {
        name: CaseName("fails_when_memory_cannot_be_read"),
        word: Word::Fails,
        input: || Ask::Root(FakeCatalog::Lists, true, 1024),
        expect: Expect::Refusal(|error| selects(Matched(matches!(error, OpenError::Memory { .. })))),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Ask::Root(catalog, memory_fails, budget) => {
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
                    search_scope: Default::default(),
                    given_inventory: Default::default(),
                })
                .map(|report| usize::from(report.session.root == RootPath(".".into())))
        }
        Ask::Declared => {
            use ryo_language::domain::files::types::FilePath;
            use ryo_language::domain::project_context::types::{
                EntryObservation, ObservedEntryState, GivenProjectInventory, ProjectInventoryCompleteness,
                ProjectSearchScope,
            };

            let scope =
                ProjectSearchScope::Declared(vec![FilePath("src".into()), FilePath("".into())]);
            let inventory = GivenProjectInventory {
                entries: vec![EntryObservation {
                    path: FilePath("src/local.py".into()),
                    state: ObservedEntryState::File,
                }],
                completeness: ProjectInventoryCompleteness::CompleteWithinDeclaredScope,
            };
            let use_case = OpenUseCaseImpl::new(
                Arc::new(LanguageFake::default()),
                Arc::new(MemoryFake {
                    asked: Budget::Bytes(MemoryBytes(1024)),
                    ..MemoryFake::default()
                }),
            );
            use_case
                .open(OpenRequest {
                    root: RootPath(".".into()),
                    repository: ryo_language::inbound::use_cases::project::types::Repository::Given,
                    search_scope: scope.clone(),
                    given_inventory: inventory.clone(),
                })
                .map(|opened| {
                    usize::from(
                        opened.session.search_scope == scope
                            && opened.session.given_inventory == inventory,
                    )
                })
        }
    }
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
