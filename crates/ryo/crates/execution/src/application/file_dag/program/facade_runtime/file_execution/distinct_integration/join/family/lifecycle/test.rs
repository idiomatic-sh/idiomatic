//! The authoritative family list drives the matrix: a new `DerivedKind` fails the exhaustive
//! `entry` match of the fixtures and is exercised on every exit. Each stored exit ends with the owner
//! generation retired and `(required, optional, unique) == (0, 0, 0)` asserted by `exercise`.
use crate::application::file_dag::derived::owner::acquisition::family::fixtures::{
    Scenario, exercise, exercise_forgetful,
    route::{exit::Exit, leave},
};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_derived::domain::request::types::DerivedKind;

#[test]
fn every_family_releases_its_last_consumer_on_success_cancellation_and_error() {
    for kind in DerivedKind::ALL.iter().copied() {
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
}

#[test]
fn every_stored_lease_survives_retirement_while_a_new_demand_refuses() {
    for kind in DerivedKind::ALL.iter().copied() {
        let actual = exercise(kind, Scenario::Held);
        assert!(
            matches!(actual, Ok(found) if found == kind),
            "{kind:?} held: {actual:?}"
        );
    }
}

/// Every exit again, through the entry production acquires the family with. The ledger is
/// asserted inside `leave`.
#[test]
fn every_family_leaves_through_its_production_entry_on_success_cancellation_and_error() {
    for kind in DerivedKind::ALL.iter().copied() {
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

/// The negative control of the release assertion: a production exit that returns with its lease
/// still charged is observed by the ledger at the exit itself, before the fixture retires the
/// generation. `ClosureAdjacency` is driven through its cursor (no pins), so it does not reach
/// the pinned exit.
#[test]
fn an_exit_that_forgets_to_release_its_lease_fails_the_control() {
    for kind in DerivedKind::ALL.iter().copied() {
        if kind == DerivedKind::ClosureAdjacency {
            continue;
        }
        for scenario in [Scenario::Succeeds, Scenario::Cancelled] {
            let leaked = exercise_forgetful(kind, scenario);
            assert!(
                matches!(leaked, Some(leak) if leak.after > leak.before),
                "{kind:?} {scenario:?}: a forgotten release must stay charged, got {leaked:?}"
            );
        }
    }
}
