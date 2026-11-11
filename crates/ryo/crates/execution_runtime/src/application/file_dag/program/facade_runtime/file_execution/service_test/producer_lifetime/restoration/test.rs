//! Restored computation holds its real backing through last use; copied delivery needs no index.
use super::fixtures::Exit;
use super::fixtures::exercise;

#[test]
fn restored_named_duplicate_delivery_releases_lookup_at_last_use_and_replays_terminal_results() {
    for exit in [Exit::Complete, Exit::SemanticFailure, Exit::RefusedBinding] {
        exercise(exit).unwrap_or_else(|error| panic!("{exit:?}: {error:?}"));
    }
}

#[test]
fn restored_named_cancellation_preserves_earlier_duplicate_deliveries_and_releases_phase() {
    exercise(Exit::CancelledDelivery).expect("actual restored delivery cancellation");
}
