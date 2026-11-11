//! Real publication scopes retain and release project batch leases.
use super::super::fixtures::scenario::Delivery as Outcome;
use super::fixtures::scenario::{Owner, Phase, exercise};
use super::fixtures::{Delivery, PhaseExit};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_values::domain::rows::errors::RowsError;
use std::borrow::Cow;

type Input = (PhaseExit, Delivery);
type Value = Phase;
type Failure = OperatingError;

/// An owner holding nothing, after this many equality builds.
const fn empty(builds: u64) -> Owner {
    Owner {
        index_available: false,
        retained: 0,
        working: 0,
        owned: 0,
        builds,
    }
}

const ROWS: Outcome = Outcome::Rows(2);
const NO_SUCH_COLUMN: Outcome =
    Outcome::Failed(OperatingError::Table(RowsError::NoSuchColumn { index: 99 }));

/// A batch that admitted work: the refusal, guard calls and evaluations, what it delivered, the
/// replays, and the transferred rows; the phase holds one equality backing until it finishes.
const fn ran(
    refusal: Option<OperatingError>,
    guards: usize,
    evaluations: u64,
    delivered: &'static [Outcome],
    replays: &'static [Outcome],
    transferred: &'static [usize],
) -> Phase {
    Phase {
        instructions_kept: true,
        at_start: empty(0),
        bound: 4,
        refusal,
        guards,
        evaluations,
        after_batch: Owner {
            index_available: false,
            retained: 0,
            working: 408,
            owned: 408,
            builds: 1,
        },
        backing_alive: Some(true),
        delivered: Cow::Borrowed(delivered),
        duplicates_equal: Some(true),
        replays: Cow::Borrowed(replays),
        replays_quiet: true,
        finished: empty(1),
        backing_released: true,
        transferred: Cow::Borrowed(transferred),
    }
}

/// The batch whose guard refused before any work: nothing was built, delivered or kept.
const REFUSED_BEFORE_WORK: Phase = Phase {
    instructions_kept: true,
    at_start: empty(0),
    bound: 4,
    refusal: Some(OperatingError::WrongBinding),
    guards: 1,
    evaluations: 0,
    after_batch: empty(0),
    backing_alive: None,
    delivered: Cow::Borrowed(&[]),
    duplicates_equal: None,
    replays: Cow::Borrowed(&[]),
    replays_quiet: true,
    finished: empty(0),
    backing_released: true,
    transferred: Cow::Borrowed(&[]),
};

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("project_batch_success_releases_actual_phase_leases"),
        word: Word::Succeeds,
        input: || (PhaseExit::Complete, Delivery::Once),
        expect: Expect::Value(ran(None, 1, 4, &[ROWS; 4], &[], &[2; 4])),
    },
    Case {
        name: CaseName("project_batch_semantic_failure_releases_actual_phase_leases"),
        word: Word::Succeeds,
        input: || (PhaseExit::SemanticFailure, Delivery::Once),
        expect: Expect::Value(ran(
            None,
            1,
            4,
            &[ROWS, ROWS, ROWS, NO_SUCH_COLUMN],
            &[],
            &[2; 3],
        )),
    },
    Case {
        name: CaseName("project_batch_refusal_before_work_releases_actual_phase_leases"),
        word: Word::Succeeds,
        input: || (PhaseExit::RefuseBeforeWork, Delivery::Once),
        expect: Expect::Value(REFUSED_BEFORE_WORK),
    },
    Case {
        name: CaseName("project_batch_refusal_before_later_cohort_releases_actual_phase_leases"),
        word: Word::Succeeds,
        input: || (PhaseExit::RefuseLaterCohort, Delivery::Once),
        expect: Expect::Value(ran(
            Some(OperatingError::WrongBinding),
            2,
            3,
            &[ROWS; 3],
            &[],
            &[2; 3],
        )),
    },
    Case {
        name: CaseName(
            "project_duplicate_delivery_preserves_terminal_failure_and_cancelled_cohort_backing",
        ),
        word: Word::Succeeds,
        input: || (PhaseExit::Complete, Delivery::Replayed),
        expect: Expect::Value(ran(None, 1, 4, &[ROWS; 4], &[ROWS; 6], &[2; 4])),
    },
    Case {
        name: CaseName("a_duplicate_delivery_replays_the_terminal_semantic_failure"),
        word: Word::Succeeds,
        input: || (PhaseExit::SemanticFailure, Delivery::Replayed),
        expect: Expect::Value(ran(
            None,
            1,
            4,
            &[ROWS, ROWS, ROWS, NO_SUCH_COLUMN],
            &[ROWS, ROWS, ROWS, ROWS, NO_SUCH_COLUMN, NO_SUCH_COLUMN],
            &[2; 3],
        )),
    },
    Case {
        name: CaseName("a_duplicate_delivery_after_a_cancelled_cohort_keeps_its_backing"),
        word: Word::Succeeds,
        input: || (PhaseExit::RefuseLaterCohort, Delivery::Replayed),
        expect: Expect::Value(ran(
            Some(OperatingError::WrongBinding),
            2,
            3,
            &[ROWS; 3],
            &[ROWS; 4],
            &[2; 3],
        )),
    },
];

/// Binds four project runs to one publication phase, runs their batch, replays when asked and
/// finishes the phase.
fn subject((exit, delivery): Input) -> Result<Value, Failure> {
    exercise(exit, delivery)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
