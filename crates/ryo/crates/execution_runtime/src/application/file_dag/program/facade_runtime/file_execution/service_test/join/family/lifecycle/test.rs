//! The authoritative family list drives the matrix: a new `DerivedKind` fails the exhaustive
//! `entry` match of the fixtures and is exercised on every exit. Each stored exit ends with the owner
//! generation retired and `(required, optional, unique) == (0, 0, 0)` asserted by `exercise`.
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_derived::domain::request::types::DerivedKind;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_store::application::file_dag::derived::owner::acquisition::family::fixtures::{
    Leaked, Scenario, exercise, exercise_forgetful,
    route::{exit::Exit, leave},
};
use std::borrow::Cow;

/// How every family is acquired and left.
#[derive(Clone, Copy, Debug)]
enum Route {
    /// Through the stored owner, leaving by this scenario.
    Stored(Scenario),
    /// Through the entry production acquires the family with, leaving by this exit; the ledger is
    /// asserted inside `leave`.
    Production(Exit),
    /// Through the stored owner by this scenario, with an exit that never releases its lease.
    Forgetful(Scenario),
}

/// What one family answered on the route.
#[derive(Clone, Debug, PartialEq)]
enum Answer {
    /// The view of this family was read.
    Read(DerivedKind),
    /// The route refused.
    Refused(OperatingError),
    /// The forgotten release stayed charged: required bytes before and after the exit.
    Leaked(Leaked),
    /// Nothing stayed charged, or the exit was not reached.
    Released,
}

type Input = Route;
type Value = Cow<'static, [(DerivedKind, Answer)]>;
type Failure = String;

/// A forgotten release that stayed charged.
const fn leaked(before: usize, after: usize) -> Answer {
    Answer::Leaked(Leaked { before, after })
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_family_releases_its_last_consumer_on_success_cancellation_and_error"),
        word: Word::Succeeds,
        input: || Route::Stored(Scenario::Succeeds),
        expect: Expect::Value(Cow::Borrowed(&[
            (DerivedKind::Equality, Answer::Read(DerivedKind::Equality)),
            (DerivedKind::Adjacency, Answer::Read(DerivedKind::Adjacency)),
            (
                DerivedKind::Membership,
                Answer::Read(DerivedKind::Membership),
            ),
            (
                DerivedKind::PartitionOrder,
                Answer::Read(DerivedKind::PartitionOrder),
            ),
            (
                DerivedKind::ClosureAdjacency,
                Answer::Read(DerivedKind::ClosureAdjacency),
            ),
            (
                DerivedKind::CycleAdjacency,
                Answer::Read(DerivedKind::CycleAdjacency),
            ),
            (
                DerivedKind::OrderedKeys,
                Answer::Read(DerivedKind::OrderedKeys),
            ),
            (
                DerivedKind::OrderedDfs,
                Answer::Read(DerivedKind::OrderedDfs),
            ),
            (
                DerivedKind::NativeLookup,
                Answer::Read(DerivedKind::NativeLookup),
            ),
        ])),
    },
    Case {
        name: CaseName("every_family_releases_its_last_consumer_on_cancellation"),
        word: Word::Succeeds,
        input: || Route::Stored(Scenario::Cancelled),
        expect: Expect::Value(Cow::Borrowed(&[
            (DerivedKind::Equality, Answer::Read(DerivedKind::Equality)),
            (DerivedKind::Adjacency, Answer::Read(DerivedKind::Adjacency)),
            (
                DerivedKind::Membership,
                Answer::Read(DerivedKind::Membership),
            ),
            (
                DerivedKind::PartitionOrder,
                Answer::Read(DerivedKind::PartitionOrder),
            ),
            (
                DerivedKind::ClosureAdjacency,
                Answer::Read(DerivedKind::ClosureAdjacency),
            ),
            (
                DerivedKind::CycleAdjacency,
                Answer::Read(DerivedKind::CycleAdjacency),
            ),
            (
                DerivedKind::OrderedKeys,
                Answer::Read(DerivedKind::OrderedKeys),
            ),
            (
                DerivedKind::OrderedDfs,
                Answer::Read(DerivedKind::OrderedDfs),
            ),
            (
                DerivedKind::NativeLookup,
                Answer::Read(DerivedKind::NativeLookup),
            ),
        ])),
    },
    Case {
        name: CaseName("every_family_refuses_a_retargeted_descriptor"),
        word: Word::Succeeds,
        input: || Route::Stored(Scenario::RetargetedDescriptor),
        expect: Expect::Value(Cow::Borrowed(&[
            (
                DerivedKind::Equality,
                Answer::Refused(OperatingError::WrongOperation),
            ),
            (
                DerivedKind::Adjacency,
                Answer::Refused(OperatingError::WrongOperation),
            ),
            (
                DerivedKind::Membership,
                Answer::Refused(OperatingError::WrongOperation),
            ),
            (
                DerivedKind::PartitionOrder,
                Answer::Refused(OperatingError::WrongOperation),
            ),
            (
                DerivedKind::ClosureAdjacency,
                Answer::Refused(OperatingError::WrongOperation),
            ),
            (
                DerivedKind::CycleAdjacency,
                Answer::Refused(OperatingError::WrongOperation),
            ),
            (
                DerivedKind::OrderedKeys,
                Answer::Refused(OperatingError::WrongOperation),
            ),
            (
                DerivedKind::OrderedDfs,
                Answer::Refused(OperatingError::WrongOperation),
            ),
            (
                DerivedKind::NativeLookup,
                Answer::Refused(OperatingError::WrongOperation),
            ),
        ])),
    },
    Case {
        name: CaseName("every_family_answers_its_build_failure"),
        word: Word::Succeeds,
        input: || Route::Stored(Scenario::BuildFails),
        expect: Expect::Value(Cow::Borrowed(&[
            (
                DerivedKind::Equality,
                Answer::Refused(OperatingError::DuplicateOrder),
            ),
            (
                DerivedKind::Adjacency,
                Answer::Refused(OperatingError::DuplicateOrder),
            ),
            (
                DerivedKind::Membership,
                Answer::Refused(OperatingError::DuplicateOrder),
            ),
            (
                DerivedKind::PartitionOrder,
                Answer::Refused(OperatingError::DuplicateOrder),
            ),
            (
                DerivedKind::ClosureAdjacency,
                Answer::Refused(OperatingError::DuplicateOrder),
            ),
            (
                DerivedKind::CycleAdjacency,
                Answer::Refused(OperatingError::DuplicateOrder),
            ),
            (
                DerivedKind::OrderedKeys,
                Answer::Refused(OperatingError::DuplicateOrder),
            ),
            (
                DerivedKind::OrderedDfs,
                Answer::Refused(OperatingError::DuplicateOrder),
            ),
            (
                DerivedKind::NativeLookup,
                Answer::Refused(OperatingError::DuplicateOrder),
            ),
        ])),
    },
    Case {
        name: CaseName("every_stored_lease_survives_retirement_while_a_new_demand_refuses"),
        word: Word::Succeeds,
        input: || Route::Stored(Scenario::Held),
        expect: Expect::Value(Cow::Borrowed(&[
            (DerivedKind::Equality, Answer::Read(DerivedKind::Equality)),
            (DerivedKind::Adjacency, Answer::Read(DerivedKind::Adjacency)),
            (
                DerivedKind::Membership,
                Answer::Read(DerivedKind::Membership),
            ),
            (
                DerivedKind::PartitionOrder,
                Answer::Read(DerivedKind::PartitionOrder),
            ),
            (
                DerivedKind::ClosureAdjacency,
                Answer::Read(DerivedKind::ClosureAdjacency),
            ),
            (
                DerivedKind::CycleAdjacency,
                Answer::Read(DerivedKind::CycleAdjacency),
            ),
            (
                DerivedKind::OrderedKeys,
                Answer::Read(DerivedKind::OrderedKeys),
            ),
            (
                DerivedKind::OrderedDfs,
                Answer::Read(DerivedKind::OrderedDfs),
            ),
            (
                DerivedKind::NativeLookup,
                Answer::Read(DerivedKind::NativeLookup),
            ),
        ])),
    },
    Case {
        name: CaseName(
            "every_family_leaves_through_its_production_entry_on_success_cancellation_and_error",
        ),
        word: Word::Succeeds,
        input: || Route::Production(Exit::Succeeds),
        expect: Expect::Value(Cow::Borrowed(&[
            (DerivedKind::Equality, Answer::Read(DerivedKind::Equality)),
            (DerivedKind::Adjacency, Answer::Read(DerivedKind::Adjacency)),
            (
                DerivedKind::Membership,
                Answer::Read(DerivedKind::Membership),
            ),
            (
                DerivedKind::PartitionOrder,
                Answer::Read(DerivedKind::PartitionOrder),
            ),
            (
                DerivedKind::ClosureAdjacency,
                Answer::Read(DerivedKind::ClosureAdjacency),
            ),
            (
                DerivedKind::CycleAdjacency,
                Answer::Read(DerivedKind::CycleAdjacency),
            ),
            (
                DerivedKind::OrderedKeys,
                Answer::Read(DerivedKind::OrderedKeys),
            ),
            (
                DerivedKind::OrderedDfs,
                Answer::Read(DerivedKind::OrderedDfs),
            ),
            (
                DerivedKind::NativeLookup,
                Answer::Read(DerivedKind::NativeLookup),
            ),
        ])),
    },
    Case {
        name: CaseName("every_family_leaves_through_its_production_entry_on_cancellation"),
        word: Word::Succeeds,
        input: || Route::Production(Exit::Cancelled),
        expect: Expect::Value(Cow::Borrowed(&[
            (DerivedKind::Equality, Answer::Read(DerivedKind::Equality)),
            (DerivedKind::Adjacency, Answer::Read(DerivedKind::Adjacency)),
            (
                DerivedKind::Membership,
                Answer::Read(DerivedKind::Membership),
            ),
            (
                DerivedKind::PartitionOrder,
                Answer::Read(DerivedKind::PartitionOrder),
            ),
            (
                DerivedKind::ClosureAdjacency,
                Answer::Read(DerivedKind::ClosureAdjacency),
            ),
            (
                DerivedKind::CycleAdjacency,
                Answer::Read(DerivedKind::CycleAdjacency),
            ),
            (
                DerivedKind::OrderedKeys,
                Answer::Read(DerivedKind::OrderedKeys),
            ),
            (
                DerivedKind::OrderedDfs,
                Answer::Read(DerivedKind::OrderedDfs),
            ),
            (
                DerivedKind::NativeLookup,
                Answer::Read(DerivedKind::NativeLookup),
            ),
        ])),
    },
    Case {
        name: CaseName("every_family_leaves_through_its_production_entry_on_error"),
        word: Word::Succeeds,
        input: || Route::Production(Exit::Errors),
        expect: Expect::Value(Cow::Borrowed(&[
            (
                DerivedKind::Equality,
                Answer::Refused(OperatingError::NotRead),
            ),
            (
                DerivedKind::Adjacency,
                Answer::Refused(OperatingError::WrongOperation),
            ),
            (
                DerivedKind::Membership,
                Answer::Refused(OperatingError::WrongOperation),
            ),
            (
                DerivedKind::PartitionOrder,
                Answer::Refused(OperatingError::NotRead),
            ),
            (
                DerivedKind::ClosureAdjacency,
                Answer::Refused(OperatingError::NotRead),
            ),
            (
                DerivedKind::CycleAdjacency,
                Answer::Refused(OperatingError::NotRead),
            ),
            (
                DerivedKind::OrderedKeys,
                Answer::Refused(OperatingError::NotRead),
            ),
            (
                DerivedKind::OrderedDfs,
                Answer::Refused(OperatingError::NotRead),
            ),
            (
                DerivedKind::NativeLookup,
                Answer::Refused(OperatingError::NotRead),
            ),
        ])),
    },
    Case {
        name: CaseName("an_exit_that_forgets_to_release_its_lease_fails_the_control"),
        word: Word::Succeeds,
        input: || Route::Forgetful(Scenario::Succeeds),
        expect: Expect::Value(Cow::Borrowed(&[
            (DerivedKind::Equality, leaked(968, 1376)),
            (DerivedKind::Adjacency, leaked(968, 1992)),
            (DerivedKind::Membership, leaked(968, 1220)),
            (DerivedKind::PartitionOrder, leaked(968, 1104)),
            (DerivedKind::ClosureAdjacency, Answer::Released),
            (DerivedKind::CycleAdjacency, leaked(968, 1304)),
            (DerivedKind::OrderedKeys, leaked(968, 1696)),
            (DerivedKind::OrderedDfs, leaked(968, 2456)),
            (DerivedKind::NativeLookup, leaked(897, 1017)),
        ])),
    },
    Case {
        name: CaseName("a_cancelled_exit_that_forgets_to_release_its_lease_fails_the_control"),
        word: Word::Succeeds,
        input: || Route::Forgetful(Scenario::Cancelled),
        expect: Expect::Value(Cow::Borrowed(&[
            (DerivedKind::Equality, leaked(968, 1376)),
            (DerivedKind::Adjacency, leaked(968, 1992)),
            (DerivedKind::Membership, leaked(968, 1220)),
            (DerivedKind::PartitionOrder, leaked(968, 1104)),
            (DerivedKind::ClosureAdjacency, Answer::Released),
            (DerivedKind::CycleAdjacency, leaked(968, 1304)),
            (DerivedKind::OrderedKeys, leaked(968, 1696)),
            (DerivedKind::OrderedDfs, leaked(968, 2456)),
            (DerivedKind::NativeLookup, leaked(897, 1017)),
        ])),
    },
];

/// Runs every family along the route, in the authoritative order.
fn subject(route: Input) -> Result<Value, Failure> {
    let answer = |kind| match route {
        Route::Stored(scenario) => read(exercise(kind, scenario)),
        Route::Production(exit) => read(leave(kind, exit)),
        Route::Forgetful(scenario) => {
            exercise_forgetful(kind, scenario).map_or(Answer::Released, Answer::Leaked)
        }
    };
    Ok(DerivedKind::ALL
        .iter()
        .map(|&kind| (kind, answer(kind)))
        .collect())
}

/// The family read, or the refusal.
fn read(answer: Result<DerivedKind, OperatingError>) -> Answer {
    answer.map_or_else(Answer::Refused, Answer::Read)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
