//! Actual external aliases retain one production backing through terminal exits.
use super::super::fixtures::scenario::Delivery;
use super::fixtures::ExternalRootExit;
use super::fixtures::exit::{Alias, Captured, Delivered, External, Finished, external_root_exit};
use crate::application::file_dag::program::runtime::types::AttemptCheckpoint;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_values::domain::rows::errors::RowsError;
use std::borrow::Cow;

type Input = ExternalRootExit;
type Value = External;
type Failure = OperatingError;

/// The second delivery and its retry, when its sink refuses the first row.
const REFUSED: Delivered = Delivered {
    refusal: Some(OperatingError::WrongBinding),
    callbacks: 1,
    retried: Some((2, true)),
};
/// The second delivery, when its sink takes both rows.
const TAKEN: Delivered = Delivered {
    refusal: None,
    callbacks: 2,
    retried: None,
};

/// The capture rule's two passes, after these repeated collected failures.
const fn captured(failures: &'static [Delivery]) -> Option<Captured> {
    Some(Captured {
        failures: Cow::Borrowed(failures),
        frontiers: Cow::Borrowed(&[1, 1]),
        decodes_transferred: true,
        quiet: true,
    })
}

const NO_SUCH_COLUMN: Delivery =
    Delivery::Failed(OperatingError::Table(RowsError::NoSuchColumn { index: 99 }));

/// What every exit observes, given the root's selected readers beyond the two Joins, the
/// equality backing's hit, the second delivery, the alias and final consumer, and the capture.
const fn external(
    retained: usize,
    hit: bool,
    second: Delivered,
    alias: Alias,
    final_consumer: Delivery,
    capture: Option<Captured>,
) -> External {
    External {
        shared_root: true,
        queued: 1,
        readers: retained + 2,
        transferred_rows: 2,
        emitted_complete: true,
        published: (AttemptCheckpoint::Published, retained),
        holds_required: true,
        backing_alive: true,
        builds: (1, hit),
        second,
        alias,
        final_consumer,
        replay_quiet: true,
        capture,
        frontier_readiness: false,
        finished: Finished {
            working: 0,
            owned: 0,
            backing_released: true,
        },
    }
}

const ROWS: Delivery = Delivery::Rows(2);
const SAME: Alias = Alias::Rows { same: true };
const NOT_RUN: Alias = Alias::Refused(OperatingError::NotRun);

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "deduplicated_external_roots_keep_real_leases_through_replay_failure_and_finish",
        ),
        word: Word::Succeeds,
        input: || ExternalRootExit::Finish,
        expect: Expect::Value(external(2, true, TAKEN, SAME, ROWS, None)),
    },
    Case {
        name: CaseName("a_rejected_external_delivery_retries_the_full_root"),
        word: Word::Succeeds,
        input: || ExternalRootExit::RejectedDelivery,
        expect: Expect::Value(external(2, true, REFUSED, SAME, ROWS, None)),
    },
    Case {
        name: CaseName("an_external_semantic_failure_keeps_the_shared_lease"),
        word: Word::Succeeds,
        input: || ExternalRootExit::SemanticFailure,
        expect: Expect::Value(external(2, true, TAKEN, SAME, NO_SUCH_COLUMN, None)),
    },
    Case {
        name: CaseName("an_external_provider_refusal_keeps_the_shared_lease"),
        word: Word::Succeeds,
        input: || ExternalRootExit::ProviderRefusal,
        expect: Expect::Value(external(
            2,
            false,
            TAKEN,
            SAME,
            Delivery::Failed(OperatingError::NotRead),
            None,
        )),
    },
    Case {
        name: CaseName("a_skipped_external_alias_is_refused_and_keeps_the_shared_lease"),
        word: Word::Succeeds,
        input: || ExternalRootExit::SkippedAlias,
        expect: Expect::Value(external(1, true, TAKEN, NOT_RUN, ROWS, None)),
    },
    Case {
        name: CaseName("an_undemanded_external_alias_is_refused_and_keeps_the_shared_lease"),
        word: Word::Succeeds,
        input: || ExternalRootExit::UndemandedAlias,
        expect: Expect::Value(external(1, true, TAKEN, NOT_RUN, ROWS, None)),
    },
    Case {
        name: CaseName(
            "captured_frontier_survives_callback_refusal_collected_failure_and_alias_cancellation",
        ),
        word: Word::Succeeds,
        input: || ExternalRootExit::CaptureCallbackRefusal,
        expect: Expect::Value(external(3, true, REFUSED, SAME, ROWS, captured(&[]))),
    },
    Case {
        name: CaseName("a_captured_frontier_survives_a_collected_failure"),
        word: Word::Succeeds,
        input: || ExternalRootExit::CaptureCollectedFailure,
        expect: Expect::Value(external(
            3,
            true,
            TAKEN,
            SAME,
            NO_SUCH_COLUMN,
            captured(&[NO_SUCH_COLUMN, NO_SUCH_COLUMN]),
        )),
    },
    Case {
        name: CaseName("a_captured_frontier_survives_a_cancelled_alias"),
        word: Word::Succeeds,
        input: || ExternalRootExit::CaptureCancelledAlias,
        expect: Expect::Value(external(2, true, TAKEN, NOT_RUN, ROWS, captured(&[]))),
    },
];

/// Runs the shared external root from preparation through delivery, aliases, capture and finish.
fn subject(exit: Input) -> Result<Value, Failure> {
    external_root_exit(exit)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
