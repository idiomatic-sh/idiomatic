//! Restored computation holds its real backing through last use; copied delivery needs no index.
use super::fixtures::Exit;
use super::fixtures::scenario::exercise;
use crate::cases::{Case, CaseName, Expect, Word, run};

fn restored_named_duplicate_delivery_releases_lookup_at_last_use_and_replays_terminal_results() {
    for exit in [Exit::Complete, Exit::SemanticFailure, Exit::RefusedBinding] {
        exercise(exit).unwrap_or_else(|error| panic!("{exit:?}: {error:?}"));
    }
}

fn restored_named_cancellation_preserves_earlier_duplicate_deliveries_and_releases_phase() {
    exercise(Exit::CancelledDelivery).expect("actual restored delivery cancellation");
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    RestoredNamedDuplicateDeliveryReleasesLookupAtLastUseAndReplaysTerminalResults,
    RestoredNamedCancellationPreservesEarlierDuplicateDeliveriesAndReleasesPhase,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "restored_named_duplicate_delivery_releases_lookup_at_last_use_and_replays_terminal_results",
        ),
        word: Word::Succeeds,
        input: || {
            Scenario::RestoredNamedDuplicateDeliveryReleasesLookupAtLastUseAndReplaysTerminalResults
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "restored_named_cancellation_preserves_earlier_duplicate_deliveries_and_releases_phase",
        ),
        word: Word::Succeeds,
        input: || {
            Scenario::RestoredNamedCancellationPreservesEarlierDuplicateDeliveriesAndReleasesPhase
        },
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::RestoredNamedDuplicateDeliveryReleasesLookupAtLastUseAndReplaysTerminalResults => {
            restored_named_duplicate_delivery_releases_lookup_at_last_use_and_replays_terminal_results();
            Ok(())
        }
        Scenario::RestoredNamedCancellationPreservesEarlierDuplicateDeliveriesAndReleasesPhase => {
            restored_named_cancellation_preserves_earlier_duplicate_deliveries_and_releases_phase();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
