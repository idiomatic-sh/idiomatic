//! Actual external aliases retain one production backing through terminal exits.
use super::fixtures::ExternalRootExit;
use super::fixtures::exit::external_root_exit;

#[test]
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

#[test]
fn captured_frontier_survives_callback_refusal_collected_failure_and_alias_cancellation() {
    for exit in [
        ExternalRootExit::CaptureCallbackRefusal,
        ExternalRootExit::CaptureCollectedFailure,
        ExternalRootExit::CaptureCancelledAlias,
    ] {
        external_root_exit(exit).unwrap_or_else(|error| panic!("{exit:?}: {error:?}"));
    }
}
