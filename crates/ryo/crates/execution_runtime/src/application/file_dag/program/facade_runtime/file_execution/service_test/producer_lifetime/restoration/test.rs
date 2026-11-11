//! Restored computation holds its real backing through last use; copied delivery needs no index.
use super::super::fixtures::scenario::{Delivery, Release};
use super::fixtures::Exit;
use super::fixtures::captured::Built;
use super::fixtures::scenario::{Held, Reading, Restored, exercise};
use crate::application::file_dag::program::runtime::types::AttemptCheckpoint;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::rows::errors::RowsError;
use std::borrow::Cow;

type Input = Exit;
type Value = Restored;
type Failure = OperatingError;

/// What every exit observes, given the batch's refusal, the equality builds, its deliveries and the
/// replays.
const fn restored(
    refusal: Option<OperatingError>,
    equality_builds: u64,
    delivered: &'static [Delivery],
    replays: &'static [Delivery],
) -> Restored {
    Restored {
        built: Built {
            frontiers: 1,
            frontier_readiness: true,
            producer_no_query: true,
            observations: 1,
            member_rows: 0,
            member_kinds: Cow::Borrowed(&[CellKind::File, CellKind::Number, CellKind::Number]),
        },
        union_phases: Cow::Borrowed(&[StepScope::CrossFile; 4]),
        index_available: false,
        reading: Reading {
            upstream: true,
            depends: true,
            equality_inputs: 1,
            join_first: true,
            reads_producer: true,
        },
        refusal,
        delivered: Cow::Borrowed(delivered),
        duplicates_equal: true,
        release_events: 2,
        release: Release {
            remaining: (1, 0),
            attempt: (AttemptCheckpoint::Published, AttemptCheckpoint::Terminal),
            alive: (true, false),
            holds_bytes: (true, false),
            paired: true,
        },
        observed_join: true,
        evaluated: true,
        equality_builds,
        after_batch: Held {
            working: 0,
            kept: 0,
            backing_released: true,
        },
        replays: Cow::Borrowed(replays),
        replays_quiet: true,
        finished: Held {
            working: 0,
            kept: 0,
            backing_released: true,
        },
        finish_builds_nothing: true,
        transferred_rows: 2,
    }
}

const NO_SUCH_COLUMN: Delivery =
    Delivery::Failed(OperatingError::Table(RowsError::NoSuchColumn { index: 99 }));
const WRONG_BINDING: Delivery = Delivery::Failed(OperatingError::WrongBinding);
const ROWS: Delivery = Delivery::Rows(2);

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "restored_named_duplicate_delivery_releases_lookup_at_last_use_and_replays_terminal_results",
        ),
        word: Word::Succeeds,
        input: || Exit::Complete,
        expect: Expect::Value(restored(None, 2, &[ROWS; 4], &[ROWS; 6])),
    },
    Case {
        name: CaseName("a_restored_semantic_failure_replays_its_terminal_failure"),
        word: Word::Succeeds,
        input: || Exit::SemanticFailure,
        expect: Expect::Value(restored(
            None,
            2,
            &[ROWS, ROWS, ROWS, NO_SUCH_COLUMN],
            &[ROWS, ROWS, ROWS, ROWS, NO_SUCH_COLUMN, NO_SUCH_COLUMN],
        )),
    },
    Case {
        name: CaseName("a_restored_refused_binding_replays_its_terminal_refusal"),
        word: Word::Succeeds,
        input: || Exit::RefusedBinding,
        expect: Expect::Value(restored(
            None,
            1,
            &[ROWS, ROWS, ROWS, WRONG_BINDING],
            &[ROWS, ROWS, ROWS, ROWS, WRONG_BINDING, WRONG_BINDING],
        )),
    },
    Case {
        name: CaseName(
            "restored_named_cancellation_preserves_earlier_duplicate_deliveries_and_releases_phase",
        ),
        word: Word::Succeeds,
        input: || Exit::CancelledDelivery,
        expect: Expect::Value(restored(
            Some(OperatingError::NotRead),
            1,
            &[ROWS; 3],
            &[ROWS; 4],
        )),
    },
];

/// Runs the restored project's four rules through one batch, replays them and finishes the phase.
fn subject(exit: Input) -> Result<Value, Failure> {
    exercise(exit)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
