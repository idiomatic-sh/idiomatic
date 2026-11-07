//! The `storing` service's cases: the repository listed in the order of its paths beside what the last run kept, and each port's
//! failure answered as its own.
use super::errors::StoringError;
use super::service::Storing;
use crate::cases::{Case, CaseName, Expect, Matched, Word, run, selects};
use crate::domain::render::types::Answering;
use crate::outbound::corpus::fixtures::fake::CorpusFake;
use crate::outbound::lint_store::fixtures::fake::LintStoreFake;
use crate::outbound::memory::fixtures::fake::MemoryFake;
use crate::outbound::scheduler::fixtures::fake::SchedulerFake;
use crate::outbound::scheduler::types::WorkerCount;
use ryo_execution::outbound::digest::fixtures::fake::DigestFake;
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::files::service::input_path;
use ryo_values::domain::files::types::{CorpusFile, ExecutablePermission, FileBytes};
use std::sync::Arc;

/// Which port of the listing a row makes fail.
#[derive(Clone, Copy, Debug)]
enum Failing {
    /// None: the listing answers.
    Nothing,
    /// The corpus refuses to list.
    Corpus,
    /// The store cannot be read.
    Store,
    /// The memory budget cannot be read.
    Memory,
    /// The machine has one byte to spare.
    Budget,
    /// The scheduler refuses to start.
    Scheduler,
}

/// The order the listing answers its files in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Order {
    /// Each path after the one before it.
    ByPath,
    /// Some other order.
    Other,
}

type Input = Failing;
type Value = (usize, Order);
type Failure = StoringError;

/// A file of the fake corpus at `path`.
fn file(path: &str) -> CorpusFile {
    CorpusFile {
        executable: ExecutablePermission::Unknown,
        path: FilePath(path.into()),
        bytes: FileBytes(b"text".to_vec()),
    }
}

fn subject(failing: Input) -> Result<Value, Failure> {
    let storing = Storing::new(
        Arc::new(CorpusFake {
            files: vec![file("b.txt"), file("a.txt")],
            fails: matches!(failing, Failing::Corpus),
            ..CorpusFake::default()
        }),
        Arc::new(DigestFake),
        Arc::new(LintStoreFake {
            unreadable: matches!(failing, Failing::Store),
            ..LintStoreFake::default()
        }),
        Arc::new(MemoryFake {
            fails: matches!(failing, Failing::Memory),
            available: if matches!(failing, Failing::Budget) { 1 } else { 1 << 30 },
            ..MemoryFake::default()
        }),
        Arc::new(SchedulerFake {
            fails: matches!(failing, Failing::Scheduler),
        }),
        WorkerCount(1),
    );
    let listing = storing.listing(&Answering::Findings)?;
    let paths: Vec<&FilePath> = listing.files.iter().map(input_path).collect();
    let order = if paths.windows(2).all(|pair| pair[0] < pair[1]) {
        Order::ByPath
    } else {
        Order::Other
    };
    Ok((paths.len(), order))
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_files_are_listed_in_the_order_of_their_paths"),
        word: Word::Succeeds,
        input: || Failing::Nothing,
        expect: Expect::Value((2, Order::ByPath)),
    },
    Case {
        name: CaseName("a_corpus_that_cannot_list_fails_the_listing"),
        word: Word::Fails,
        input: || Failing::Corpus,
        expect: Expect::Refusal(|error| selects(Matched(matches!(error, StoringError::Corpus(_))))),
    },
    Case {
        name: CaseName("a_store_that_cannot_be_read_fails_the_listing"),
        word: Word::Fails,
        input: || Failing::Store,
        expect: Expect::Refusal(|error| selects(Matched(matches!(error, StoringError::Store(_))))),
    },
    Case {
        name: CaseName("a_budget_that_cannot_be_read_fails_the_listing"),
        word: Word::Fails,
        input: || Failing::Memory,
        expect: Expect::Refusal(|error| selects(Matched(matches!(error, StoringError::Memory(_))))),
    },
    Case {
        name: CaseName("a_repository_larger_than_the_budget_is_refused"),
        word: Word::Refuses,
        input: || Failing::Budget,
        expect: Expect::Refusal(|error| selects(Matched(matches!(error, StoringError::Budget(_))))),
    },
    Case {
        name: CaseName("a_scheduler_that_cannot_start_fails_the_listing"),
        word: Word::Fails,
        input: || Failing::Scheduler,
        expect: Expect::Refusal(|error| {
            selects(Matched(matches!(error, StoringError::Scheduler(_))))
        }),
    },
];

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
