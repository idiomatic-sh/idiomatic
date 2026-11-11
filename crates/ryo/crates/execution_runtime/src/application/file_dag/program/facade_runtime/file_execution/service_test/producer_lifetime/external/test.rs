//! Actual external aliases retain one production backing through terminal exits.
use super::fixtures::ExternalRootExit;
use super::fixtures::exit::external_root_exit;
use crate::cases::{Case, CaseName, Expect, Word, run};

fn deduplicated_external_roots_keep_real_leases_through_replay_failure_and_finish() {
    for exit in [
        ExternalRootExit::Finish,
        ExternalRootExit::RejectedDelivery,
        ExternalRootExit::SemanticFailure,
        ExternalRootExit::ProviderRefusal,
        ExternalRootExit::SkippedAlias,
        ExternalRootExit::UndemandedAlias,
    ] {
        external_root_exit(exit).unwrap_or_else(|error| panic!("{exit:?}: {error:?}"));
    }
}

fn captured_frontier_survives_callback_refusal_collected_failure_and_alias_cancellation() {
    for exit in [
        ExternalRootExit::CaptureCallbackRefusal,
        ExternalRootExit::CaptureCollectedFailure,
        ExternalRootExit::CaptureCancelledAlias,
    ] {
        external_root_exit(exit).unwrap_or_else(|error| panic!("{exit:?}: {error:?}"));
    }
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    DeduplicatedExternalRootsKeepRealLeasesThroughReplayFailureAndFinish,
    CapturedFrontierSurvivesCallbackRefusalCollectedFailureAndAliasCancellation,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "deduplicated_external_roots_keep_real_leases_through_replay_failure_and_finish",
        ),
        word: Word::Succeeds,
        input: || Scenario::DeduplicatedExternalRootsKeepRealLeasesThroughReplayFailureAndFinish,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "captured_frontier_survives_callback_refusal_collected_failure_and_alias_cancellation",
        ),
        word: Word::Succeeds,
        input: || {
            Scenario::CapturedFrontierSurvivesCallbackRefusalCollectedFailureAndAliasCancellation
        },
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::DeduplicatedExternalRootsKeepRealLeasesThroughReplayFailureAndFinish => {
            deduplicated_external_roots_keep_real_leases_through_replay_failure_and_finish();
            Ok(())
        }
        Scenario::CapturedFrontierSurvivesCallbackRefusalCollectedFailureAndAliasCancellation => {
            captured_frontier_survives_callback_refusal_collected_failure_and_alias_cancellation();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
