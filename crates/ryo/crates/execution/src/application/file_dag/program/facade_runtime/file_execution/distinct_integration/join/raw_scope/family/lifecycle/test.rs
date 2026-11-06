//! The authoritative family list drives the matrix: a new `DerivedKind` fails the exhaustive
//! `route` match below and is exercised on every exit. Each stored exit ends with the owner
//! generation retired and `(required, optional, unique) == (0, 0, 0)` asserted by `exercise`.
use super::super::super::fixtures::{RawCase, observe};
use crate::application::file_dag::derived::owner::acquisition::family::fixtures::{
    Scenario, exercise,
    route::{
        exit::{Entry, Exit, entry},
        leave,
    },
};
use crate::domain::operating::errors::OperatingError;
use ryo_derived::domain::contract::types::DerivedKind;
use ryo_values::domain::rows::types::RowCount;

enum Route {
    /// Issued by stored admission; the closure partition goes through its cursor.
    Stored,
    /// Finite invocation work behind the kernel permit; only the compiled native join reaches it.
    Raw,
}
/// Exhaustive on purpose: a new family must choose its route before this file compiles.
fn route(kind: DerivedKind) -> Route {
    match kind {
        DerivedKind::RawKeyOrder => Route::Raw,
        DerivedKind::Equality
        | DerivedKind::PredicateMatches
        | DerivedKind::CandidatePostings
        | DerivedKind::DistinctRepresentatives
        | DerivedKind::Adjacency
        | DerivedKind::Membership
        | DerivedKind::PartitionOrder
        | DerivedKind::ClosureAdjacency
        | DerivedKind::CycleAdjacency
        | DerivedKind::OrderedKeys
        | DerivedKind::OrderedDfs
        | DerivedKind::NativeLookup => Route::Stored,
    }
}

#[test]
fn every_family_releases_its_last_consumer_on_success_cancellation_and_error() {
    for kind in DerivedKind::ALL.iter().copied() {
        match route(kind) {
            Route::Stored => {
                for scenario in [Scenario::Succeeds, Scenario::Cancelled] {
                    let actual = exercise(kind, scenario);
                    assert!(
                        matches!(actual, Ok(found) if found == kind),
                        "{kind:?} {scenario:?}: {actual:?}"
                    );
                }
                let refused = exercise(kind, Scenario::RetargetedDescriptor);
                assert!(
                    matches!(refused, Err(OperatingError::WrongOperation)),
                    "{kind:?} error exit: {refused:?}"
                );
                let failed = exercise(kind, Scenario::BuildFails);
                assert!(
                    matches!(failed, Err(OperatingError::DuplicateOrder)),
                    "{kind:?} build failure: {failed:?}"
                );
            }
            Route::Raw => {
                assert!(
                    matches!(observe(RawCase::Complete), Ok(RowCount(3))),
                    "{kind:?} success"
                );
                assert!(
                    matches!(observe(RawCase::SinkFailure), Err(OperatingError::NotRun)),
                    "{kind:?} cancellation"
                );
                assert!(
                    matches!(
                        observe(RawCase::DerivedFailure),
                        Err(OperatingError::DuplicateOrder)
                    ),
                    "{kind:?} port error"
                );
                assert!(
                    observe(RawCase::SemanticFailure).is_err(),
                    "{kind:?} callback error"
                );
            }
        }
    }
}

#[test]
fn every_stored_lease_survives_retirement_while_a_new_demand_refuses() {
    for kind in DerivedKind::ALL.iter().copied() {
        if matches!(route(kind), Route::Stored) {
            let actual = exercise(kind, Scenario::Held);
            assert!(
                matches!(actual, Ok(found) if found == kind),
                "{kind:?} held: {actual:?}"
            );
        }
    }
}

/// Every exit again, through the entry production acquires the family with. The ledger is
/// asserted inside `leave`. `RawKeyOrder` needs a `KernelPermit`: its success, cancellation and
/// error exits are the `raw_scope::test` cases named in the contract matrix.
#[test]
fn every_family_leaves_through_its_production_entry_on_success_cancellation_and_error() {
    for kind in DerivedKind::ALL.iter().copied() {
        if entry(kind) == Entry::Permit {
            continue;
        }
        for exit in [Exit::Succeeds, Exit::Cancelled] {
            let actual = leave(kind, exit);
            assert!(
                matches!(actual, Ok(found) if found == kind),
                "{kind:?} {exit:?}: {actual:?}"
            );
        }
        let refused = leave(kind, Exit::Errors);
        assert!(refused.is_err(), "{kind:?} error exit: {refused:?}");
    }
}
