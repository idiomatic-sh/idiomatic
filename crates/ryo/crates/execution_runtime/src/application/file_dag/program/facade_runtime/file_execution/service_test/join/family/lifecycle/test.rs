//! The authoritative family list drives the matrix: a new `DerivedKind` fails the exhaustive
//! `entry` match of the fixtures and is exercised on every exit. Each row states, for every family,
//! what its route read or refused, what it saw on the way, the lease its exit left charged and the
//! `(required, optional, unique)` bytes the owner still charges once its generation retired.
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_derived::domain::request::types::DerivedKind as K;
use ryo_execution_domain::domain::operating::errors::OperatingError as E;
use ryo_execution_store::application::file_dag::derived::owner::acquisition::family::fixtures::{
    Departure, Leaked,
    departed::{Departed, Refused, Seen},
    exercise, exercise_forgetful,
    route::{exit::Exit, leave},
};
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use std::borrow::Cow;

/// How every family is acquired and left.
#[derive(Clone, Copy, Debug)]
enum Route {
    /// Through the stored owner, leaving by this departure.
    Stored(Departure),
    /// Through the entry production acquires the family with, leaving by this exit.
    Production(Exit),
    /// Through the stored owner by this departure, with an exit that never releases its lease.
    Forgetful(Departure),
}

/// What one family answered on the route.
#[derive(Clone, Debug, PartialEq)]
enum Answer {
    /// The view of this family was read.
    Read(Departed),
    /// The route refused.
    Refused(Refused),
}

type Input = Route;
type Value = Cow<'static, [(K, Answer)]>;
type Failure = String;

/// The family read, having seen `along`, with no lease left at the exit or after retirement.
const fn read(kind: K, along: &'static [Seen]) -> (K, Answer) {
    let departed = Departed {
        kind,
        along: Cow::Borrowed(along),
        leaked: None,
        retired: (0, 0, 0),
    };
    (kind, Answer::Read(departed))
}

/// The family refused with `error`, having seen `along`, with no lease left at the exit or after
/// retirement.
const fn refused(kind: K, error: E, along: &'static [Seen]) -> (K, Answer) {
    let refused = Refused {
        error,
        along: Cow::Borrowed(along),
        leaked: None,
        retired: (0, 0, 0),
    };
    (kind, Answer::Refused(refused))
}

/// The family read, with the exit leaving its lease charged: required bytes before and after it.
const fn leaked(kind: K, before: usize, after: usize) -> (K, Answer) {
    let departed = Departed {
        kind,
        along: Cow::Borrowed(&[]),
        leaked: Some(Leaked { before, after }),
        retired: (0, 0, 0),
    };
    (kind, Answer::Read(departed))
}

/// The key of the closure cursor's first partition.
const KEY_0: Seen = Seen::Key(Cow::Borrowed(&[Cell::Number(Integer(0))]));
/// The key of the closure cursor's second partition.
const KEY_1: Seen = Seen::Key(Cow::Borrowed(&[Cell::Number(Integer(1))]));
/// The closure cursor's walk: two partitions, then none.
const WALKED: &[Seen] = &[
    Seen::Advanced,
    KEY_0,
    Seen::Advanced,
    KEY_1,
    Seen::Exhausted,
];
/// The closure cursor's first partition, then the withdrawal.
const FIRST_PARTITION: &[Seen] = &[Seen::Advanced, KEY_0];
/// The first partition, held past retirement, and the refused next one.
const HELD_PARTITION: &[Seen] = &[Seen::Advanced, KEY_0, KEY_0, Seen::Refused(E::NotRead)];

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_family_releases_its_last_consumer_on_success_cancellation_and_error"),
        word: Word::Succeeds,
        input: || Route::Stored(Departure::Succeeds),
        expect: Expect::Value(Cow::Borrowed(
            const {
                &[
                    read(K::Equality, &[]),
                    read(K::Adjacency, &[]),
                    read(K::Membership, &[]),
                    read(K::PartitionOrder, &[]),
                    read(K::ClosureAdjacency, WALKED),
                    read(K::CycleAdjacency, &[]),
                    read(K::OrderedKeys, &[]),
                    read(K::OrderedDfs, &[]),
                    read(K::NativeLookup, &[]),
                ]
            },
        )),
    },
    Case {
        name: CaseName("every_family_releases_its_last_consumer_on_cancellation"),
        word: Word::Succeeds,
        input: || Route::Stored(Departure::Cancelled),
        expect: Expect::Value(Cow::Borrowed(
            const {
                &[
                    read(K::Equality, &[]),
                    read(K::Adjacency, &[]),
                    read(K::Membership, &[]),
                    read(K::PartitionOrder, &[]),
                    read(K::ClosureAdjacency, FIRST_PARTITION),
                    read(K::CycleAdjacency, &[]),
                    read(K::OrderedKeys, &[]),
                    read(K::OrderedDfs, &[]),
                    read(K::NativeLookup, &[]),
                ]
            },
        )),
    },
    Case {
        name: CaseName("every_family_refuses_a_retargeted_descriptor"),
        word: Word::Succeeds,
        input: || Route::Stored(Departure::RetargetedDescriptor),
        expect: Expect::Value(Cow::Borrowed(
            const {
                &[
                    refused(K::Equality, E::WrongOperation, &[]),
                    refused(K::Adjacency, E::WrongOperation, &[]),
                    refused(K::Membership, E::WrongOperation, &[]),
                    refused(K::PartitionOrder, E::WrongOperation, &[]),
                    refused(K::ClosureAdjacency, E::WrongOperation, &[]),
                    refused(K::CycleAdjacency, E::WrongOperation, &[]),
                    refused(K::OrderedKeys, E::WrongOperation, &[]),
                    refused(K::OrderedDfs, E::WrongOperation, &[]),
                    refused(K::NativeLookup, E::WrongOperation, &[]),
                ]
            },
        )),
    },
    Case {
        name: CaseName("every_family_answers_its_build_failure"),
        word: Word::Succeeds,
        input: || Route::Stored(Departure::BuildFails),
        expect: Expect::Value(Cow::Borrowed(
            const {
                &[
                    refused(K::Equality, E::DuplicateOrder, &[]),
                    refused(K::Adjacency, E::DuplicateOrder, &[]),
                    refused(K::Membership, E::DuplicateOrder, &[]),
                    refused(K::PartitionOrder, E::DuplicateOrder, &[]),
                    refused(K::ClosureAdjacency, E::DuplicateOrder, &[]),
                    refused(K::CycleAdjacency, E::DuplicateOrder, &[]),
                    refused(K::OrderedKeys, E::DuplicateOrder, &[]),
                    refused(K::OrderedDfs, E::DuplicateOrder, &[]),
                    refused(K::NativeLookup, E::DuplicateOrder, &[]),
                ]
            },
        )),
    },
    Case {
        name: CaseName("every_stored_lease_survives_retirement_while_a_new_demand_refuses"),
        word: Word::Succeeds,
        input: || Route::Stored(Departure::Held),
        expect: Expect::Value(Cow::Borrowed(
            const {
                &[
                    read(
                        K::Equality,
                        &[Seen::Read(K::Equality), Seen::Refused(E::NotRead)],
                    ),
                    read(
                        K::Adjacency,
                        &[Seen::Read(K::Adjacency), Seen::Refused(E::NotRead)],
                    ),
                    read(
                        K::Membership,
                        &[Seen::Read(K::Membership), Seen::Refused(E::NotRead)],
                    ),
                    read(
                        K::PartitionOrder,
                        &[Seen::Read(K::PartitionOrder), Seen::Refused(E::NotRead)],
                    ),
                    read(K::ClosureAdjacency, HELD_PARTITION),
                    read(
                        K::CycleAdjacency,
                        &[Seen::Read(K::CycleAdjacency), Seen::Refused(E::NotRead)],
                    ),
                    read(
                        K::OrderedKeys,
                        &[Seen::Read(K::OrderedKeys), Seen::Refused(E::NotRead)],
                    ),
                    read(
                        K::OrderedDfs,
                        &[Seen::Read(K::OrderedDfs), Seen::Refused(E::NotRead)],
                    ),
                    read(
                        K::NativeLookup,
                        &[Seen::Read(K::NativeLookup), Seen::Refused(E::NotRead)],
                    ),
                ]
            },
        )),
    },
    Case {
        name: CaseName(
            "every_family_leaves_through_its_production_entry_on_success_cancellation_and_error",
        ),
        word: Word::Succeeds,
        input: || Route::Production(Exit::Succeeds),
        expect: Expect::Value(Cow::Borrowed(
            const {
                &[
                    read(K::Equality, &[Seen::Read(K::Equality)]),
                    read(K::Adjacency, &[]),
                    read(K::Membership, &[]),
                    read(K::PartitionOrder, &[Seen::Advanced]),
                    read(K::ClosureAdjacency, &[Seen::Advanced]),
                    read(K::CycleAdjacency, &[Seen::Read(K::CycleAdjacency)]),
                    read(K::OrderedKeys, &[Seen::Read(K::OrderedKeys)]),
                    read(K::OrderedDfs, &[Seen::Read(K::OrderedDfs)]),
                    read(K::NativeLookup, &[Seen::Read(K::NativeLookup)]),
                ]
            },
        )),
    },
    Case {
        name: CaseName("every_family_leaves_through_its_production_entry_on_cancellation"),
        word: Word::Succeeds,
        input: || Route::Production(Exit::Cancelled),
        expect: Expect::Value(Cow::Borrowed(
            const {
                &[
                    read(K::Equality, &[]),
                    read(K::Adjacency, &[]),
                    read(K::Membership, &[]),
                    read(K::PartitionOrder, &[]),
                    read(K::ClosureAdjacency, &[]),
                    read(K::CycleAdjacency, &[]),
                    read(K::OrderedKeys, &[]),
                    read(K::OrderedDfs, &[]),
                    read(K::NativeLookup, &[]),
                ]
            },
        )),
    },
    Case {
        name: CaseName("every_family_leaves_through_its_production_entry_on_error"),
        word: Word::Succeeds,
        input: || Route::Production(Exit::Errors),
        expect: Expect::Value(Cow::Borrowed(
            const {
                &[
                    refused(K::Equality, E::NotRead, &[Seen::Refused(E::WrongOperation)]),
                    refused(K::Adjacency, E::WrongOperation, &[]),
                    refused(K::Membership, E::WrongOperation, &[]),
                    refused(K::PartitionOrder, E::NotRead, &[]),
                    refused(K::ClosureAdjacency, E::NotRead, &[]),
                    refused(
                        K::CycleAdjacency,
                        E::NotRead,
                        &[Seen::Refused(E::WrongOperation)],
                    ),
                    refused(
                        K::OrderedKeys,
                        E::NotRead,
                        &[Seen::Refused(E::WrongOperation)],
                    ),
                    refused(
                        K::OrderedDfs,
                        E::NotRead,
                        &[Seen::Refused(E::WrongOperation)],
                    ),
                    refused(K::NativeLookup, E::NotRead, &[]),
                ]
            },
        )),
    },
    Case {
        name: CaseName("an_exit_that_forgets_to_release_its_lease_fails_the_control"),
        word: Word::Succeeds,
        input: || Route::Forgetful(Departure::Succeeds),
        expect: Expect::Value(Cow::Borrowed(
            const {
                &[
                    leaked(K::Equality, 968, 1376),
                    leaked(K::Adjacency, 968, 1992),
                    leaked(K::Membership, 968, 1220),
                    leaked(K::PartitionOrder, 968, 1104),
                    read(K::ClosureAdjacency, WALKED),
                    leaked(K::CycleAdjacency, 968, 1304),
                    leaked(K::OrderedKeys, 968, 1696),
                    leaked(K::OrderedDfs, 968, 2456),
                    leaked(K::NativeLookup, 897, 1017),
                ]
            },
        )),
    },
    Case {
        name: CaseName("a_cancelled_exit_that_forgets_to_release_its_lease_fails_the_control"),
        word: Word::Succeeds,
        input: || Route::Forgetful(Departure::Cancelled),
        expect: Expect::Value(Cow::Borrowed(
            const {
                &[
                    leaked(K::Equality, 968, 1376),
                    leaked(K::Adjacency, 968, 1992),
                    leaked(K::Membership, 968, 1220),
                    leaked(K::PartitionOrder, 968, 1104),
                    read(K::ClosureAdjacency, FIRST_PARTITION),
                    leaked(K::CycleAdjacency, 968, 1304),
                    leaked(K::OrderedKeys, 968, 1696),
                    leaked(K::OrderedDfs, 968, 2456),
                    leaked(K::NativeLookup, 897, 1017),
                ]
            },
        )),
    },
];

/// Runs every family along the route, in the authoritative order.
fn subject(route: Input) -> Result<Value, Failure> {
    let answer = |kind| {
        match route {
            Route::Stored(departure) => exercise(kind, departure),
            Route::Production(exit) => leave(kind, exit),
            Route::Forgetful(departure) => exercise_forgetful(kind, departure),
        }
        .map_or_else(Answer::Refused, Answer::Read)
    };
    Ok(K::ALL.iter().map(|&kind| (kind, answer(kind))).collect())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
