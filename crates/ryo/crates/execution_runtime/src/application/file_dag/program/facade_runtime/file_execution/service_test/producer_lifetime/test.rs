//! Required derived storage follows real compiled producer consumers, including cancellation.
use super::fixtures::FinalConsumer;
use super::fixtures::scenario::{Lifetime, Release, Second, exercise};
use crate::application::file_dag::program::runtime::types::AttemptCheckpoint;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_values::domain::rows::errors::RowsError;
use std::borrow::Cow;

type Input = FinalConsumer;
type Value = Lifetime;
type Failure = OperatingError;

/// A healthy consumer's release that leaves `remaining` consumers.
const fn released(remaining: usize) -> Release {
    let last = remaining == 0;
    Release {
        remaining: (remaining + 1, remaining),
        attempt: (
            AttemptCheckpoint::Published,
            if last {
                AttemptCheckpoint::Terminal
            } else {
                AttemptCheckpoint::Published
            },
        ),
        alive: (true, !last),
        holds_bytes: (true, !last),
        paired: true,
    }
}

/// The lifetime every exit shares: one producer of two consumers, the first's rows, and a
/// producer that ends terminal with nothing held.
const fn lifetime(
    remaining_after_exclusion: Option<usize>,
    builds: (u64, u64, bool),
    second: Second,
    releases: &'static [Release],
) -> Lifetime {
    Lifetime {
        shared_consumers: Some(2),
        remaining_after_exclusion,
        first: Cow::Borrowed(&[2, 9, 3, 7]),
        builds,
        second,
        final_rereads: 0,
        releases: Cow::Borrowed(releases),
        one_backing: true,
        final_state: (AttemptCheckpoint::Terminal, 0),
        working_bytes: 0,
        first_survives: true,
        backing_released: true,
        owned_bytes: 0,
    }
}

const BOTH_RELEASED: &[Release] = &[released(1), released(0)];

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("shared_published_producer_keeps_index_until_its_final_real_consumer"),
        word: Word::Succeeds,
        input: || FinalConsumer::Completes,
        expect: Expect::Value(lifetime(
            None,
            (1, 1, true),
            Second::Rows(Cow::Borrowed(&[9, 2, 7, 3])),
            BOTH_RELEASED,
        )),
    },
    Case {
        name: CaseName(
            "terminal_shape_failure_preserves_actual_reader_release_and_transferred_output",
        ),
        word: Word::Succeeds,
        input: || FinalConsumer::InvalidTerminal,
        expect: Expect::Value(lifetime(
            None,
            (1, 1, true),
            Second::Failed(OperatingError::Table(RowsError::NoSuchColumn { index: 99 })),
            BOTH_RELEASED,
        )),
    },
    Case {
        name: CaseName(
            "final_refusing_consumer_releases_zero_budget_index_without_rereading_its_producer",
        ),
        word: Word::Succeeds,
        input: || FinalConsumer::RefusingProvider,
        expect: Expect::Value(lifetime(
            None,
            (1, 1, false),
            Second::Failed(OperatingError::NotRead),
            BOTH_RELEASED,
        )),
    },
    Case {
        name: CaseName(
            "upfront_skipped_reader_releases_the_real_shared_index_after_the_remaining_reader",
        ),
        word: Word::Succeeds,
        input: || FinalConsumer::SkippedAtStart,
        expect: Expect::Value(lifetime(
            Some(1),
            (1, 1, false),
            Second::Refused(OperatingError::NotRun),
            &[released(0)],
        )),
    },
];

/// Runs both consumers of the shared producer, the second ending as the input says.
fn subject(final_consumer: Input) -> Result<Value, Failure> {
    exercise(final_consumer)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
