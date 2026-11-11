//! The authoritative family list drives the matrix: a new `DerivedKind` fails the exhaustive
//! `entry` match of the fixtures and is exercised on every exit. Each stored exit ends with the owner
//! generation retired and `(required, optional, unique) == (0, 0, 0)` asserted by `exercise`.
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_derived::domain::request::types::DerivedKind;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_store::application::file_dag::derived::owner::acquisition::family::fixtures::{
    Scenario, exercise, exercise_forgetful,
    route::{exit::Exit, leave},
};

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

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Proof {
    EveryFamilyReleasesItsLastConsumerOnSuccessCancellationAndError,
    EveryStoredLeaseSurvivesRetirementWhileANewDemandRefuses,
    EveryFamilyLeavesThroughItsProductionEntryOnSuccessCancellationAndError,
    AnExitThatForgetsToReleaseItsLeaseFailsTheControl,
}

type Input = Proof;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_family_releases_its_last_consumer_on_success_cancellation_and_error"),
        word: Word::Succeeds,
        input: || Proof::EveryFamilyReleasesItsLastConsumerOnSuccessCancellationAndError,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("every_stored_lease_survives_retirement_while_a_new_demand_refuses"),
        word: Word::Succeeds,
        input: || Proof::EveryStoredLeaseSurvivesRetirementWhileANewDemandRefuses,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "every_family_leaves_through_its_production_entry_on_success_cancellation_and_error",
        ),
        word: Word::Succeeds,
        input: || Proof::EveryFamilyLeavesThroughItsProductionEntryOnSuccessCancellationAndError,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("an_exit_that_forgets_to_release_its_lease_fails_the_control"),
        word: Word::Succeeds,
        input: || Proof::AnExitThatForgetsToReleaseItsLeaseFailsTheControl,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Proof::EveryFamilyReleasesItsLastConsumerOnSuccessCancellationAndError => {
            every_family_releases_its_last_consumer_on_success_cancellation_and_error();
            Ok(())
        }
        Proof::EveryStoredLeaseSurvivesRetirementWhileANewDemandRefuses => {
            every_stored_lease_survives_retirement_while_a_new_demand_refuses();
            Ok(())
        }
        Proof::EveryFamilyLeavesThroughItsProductionEntryOnSuccessCancellationAndError => {
            every_family_leaves_through_its_production_entry_on_success_cancellation_and_error();
            Ok(())
        }
        Proof::AnExitThatForgetsToReleaseItsLeaseFailsTheControl => {
            an_exit_that_forgets_to_release_its_lease_fails_the_control();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
