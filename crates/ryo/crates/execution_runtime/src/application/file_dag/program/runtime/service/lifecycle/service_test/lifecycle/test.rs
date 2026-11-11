//! Compiled program lifecycle proof with a real owner.
use super::super::super::fixtures::{Reentry, RuntimeProbeMode, Seen};
use super::fixtures::{Cleanup, Lived, cleanup, counted, initialized, nested, reentered, retried};
use crate::application::file_dag::program::runtime::types::DependencyCheckpoint;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::readiness::types::{ReadinessCounts, ReadinessEvents};
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use std::borrow::Cow;

/// The lifecycle path a row runs one compiled program through.
#[derive(Clone, Copy)]
enum Lifecycle {
    /// The outer root's nested demand of the inner root fails.
    NestedFailure,
    /// The root reenters its own region.
    Reentry,
    /// The root's first replay is refused by its sink, then retried.
    RefusedReplay,
    /// The root runs once under this probe mode.
    Counted(RuntimeProbeMode),
    /// Activated roots' slots are touched repeatedly.
    Touched,
    /// A retained root's repeated release meets another root's semantic failure.
    Cleanup,
}

type Input = Lifecycle;
type Value = Lived;
type Failure = OperatingError;

const WRONG_BINDING: Option<OperatingError> = Some(OperatingError::WrongBinding);
const NOT_RUN: Option<OperatingError> = Some(OperatingError::NotRun);

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("nested_demand_preserves_both_frames_and_exact_semantic_failure"),
        word: Word::Succeeds,
        input: || Lifecycle::NestedFailure,
        expect: Expect::Value(Lived::Nested {
            invocations: [WRONG_BINDING; 2],
            slots: Cow::Borrowed(&[(WRONG_BINDING, true), (WRONG_BINDING, true)]),
            evaluations: 2,
            seen: Cow::Borrowed(&[Seen::Nested {
                outer_is_rule_root: true,
                inner_failed: true,
                ordered_row: Cow::Borrowed(&[Cell::Number(Integer(100))]),
            }]),
        }),
    },
    Case {
        name: CaseName("same_region_reentry_refuses_without_corrupting_successful_publication"),
        word: Word::Succeeds,
        input: || Lifecycle::Reentry,
        expect: Expect::Value(Lived::Reentered {
            published: [true; 2],
            reentry: Reentry::Refused,
            evaluations: 1,
        }),
    },
    Case {
        name: CaseName("downstream_error_keeps_publication_and_retry_does_not_repeat_semantics"),
        word: Word::Succeeds,
        input: || Lifecycle::RefusedReplay,
        expect: Expect::Value(Lived::Retried {
            refusal: WRONG_BINDING,
            published: true,
            frames_empty: true,
            replayed: 1,
            evaluations: 1,
        }),
    },
    Case {
        name: CaseName(
            "a_region_evaluated_in_its_preparation_is_counted_once_as_a_dispatched_one_is",
        ),
        word: Word::Succeeds,
        input: || Lifecycle::Counted(RuntimeProbeMode::Plain),
        expect: Expect::Value(Lived::Counted {
            published: true,
            evaluations: 1,
            counts: ReadinessCounts {
                preparations: ReadinessEvents(1),
                evaluations: ReadinessEvents(1),
                suspensions: ReadinessEvents(0),
                wakeups: ReadinessEvents(0),
                presence_absent: ReadinessEvents(0),
            },
        }),
    },
    Case {
        name: CaseName("a_region_evaluated_by_a_dispatched_action_is_counted_once"),
        word: Word::Succeeds,
        input: || Lifecycle::Counted(RuntimeProbeMode::Joined),
        expect: Expect::Value(Lived::Counted {
            published: true,
            evaluations: 1,
            counts: ReadinessCounts {
                preparations: ReadinessEvents(1),
                evaluations: ReadinessEvents(1),
                suspensions: ReadinessEvents(0),
                wakeups: ReadinessEvents(0),
                presence_absent: ReadinessEvents(0),
            },
        }),
    },
    Case {
        name: CaseName("reached_slots_initialize_reader_obligations_once"),
        word: Word::Succeeds,
        input: || Lifecycle::Touched,
        expect: Expect::Value(Lived::Initialized {
            first: 1,
            second: 2,
            slots: 2,
        }),
    },
    Case {
        name: CaseName(
            "repeated_consumer_release_is_atomic_and_primary_semantic_failure_wins_cleanup",
        ),
        word: Word::Succeeds,
        input: || Lifecycle::Cleanup,
        expect: Expect::Value(Lived::Cleanup(Cleanup {
            retained: (2, DependencyCheckpoint::Consumed),
            releases: [NOT_RUN; 2],
            release_kept: true,
            failing: [WRONG_BINDING; 2],
            evaluations: (1, 1),
            retained_kept: true,
            replayed: 1,
            finish: NOT_RUN,
            seen: Cow::Borrowed(&[
                Seen::Release {
                    refused: true,
                    checkpoint_kept: true,
                },
                Seen::Release {
                    refused: true,
                    checkpoint_kept: true,
                },
            ]),
        })),
    },
];

/// Runs the compiled program through the row's lifecycle path.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Lifecycle::NestedFailure => nested(),
        Lifecycle::Reentry => reentered(),
        Lifecycle::RefusedReplay => retried(),
        Lifecycle::Counted(mode) => counted(mode),
        Lifecycle::Touched => initialized(),
        Lifecycle::Cleanup => cleanup(),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
