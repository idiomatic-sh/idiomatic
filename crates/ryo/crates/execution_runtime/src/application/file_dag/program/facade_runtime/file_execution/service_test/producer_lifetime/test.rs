//! Required derived storage follows real compiled producer consumers, including cancellation.
use super::fixtures::FinalConsumer;
use super::fixtures::scenario::exercise;
use crate::cases::{Case, CaseName, Expect, Word, run};

fn shared_published_producer_keeps_index_until_its_final_real_consumer() {
    exercise(FinalConsumer::Completes).expect("actual compiled producer lifetime");
}
fn terminal_shape_failure_preserves_actual_reader_release_and_transferred_output() {
    exercise(FinalConsumer::InvalidTerminal).expect("actual compiled failure cancellation");
}

fn final_refusing_consumer_releases_zero_budget_index_without_rereading_its_producer() {
    for final_consumer in [FinalConsumer::RefusingProvider] {
        exercise(final_consumer).expect("actual compiled final consumer cancellation");
    }
}

fn upfront_skipped_reader_releases_the_real_shared_index_after_the_remaining_reader() {
    exercise(FinalConsumer::SkippedAtStart)
        .expect("actual zero-budget final reader release before file finish");
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    SharedPublishedProducerKeepsIndexUntilItsFinalRealConsumer,
    TerminalShapeFailurePreservesActualReaderReleaseAndTransferredOutput,
    FinalRefusingConsumerReleasesZeroBudgetIndexWithoutRereadingItsProducer,
    UpfrontSkippedReaderReleasesTheRealSharedIndexAfterTheRemainingReader,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("shared_published_producer_keeps_index_until_its_final_real_consumer"),
        word: Word::Succeeds,
        input: || Scenario::SharedPublishedProducerKeepsIndexUntilItsFinalRealConsumer,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "terminal_shape_failure_preserves_actual_reader_release_and_transferred_output",
        ),
        word: Word::Succeeds,
        input: || Scenario::TerminalShapeFailurePreservesActualReaderReleaseAndTransferredOutput,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "final_refusing_consumer_releases_zero_budget_index_without_rereading_its_producer",
        ),
        word: Word::Succeeds,
        input: || Scenario::FinalRefusingConsumerReleasesZeroBudgetIndexWithoutRereadingItsProducer,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "upfront_skipped_reader_releases_the_real_shared_index_after_the_remaining_reader",
        ),
        word: Word::Succeeds,
        input: || Scenario::UpfrontSkippedReaderReleasesTheRealSharedIndexAfterTheRemainingReader,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::SharedPublishedProducerKeepsIndexUntilItsFinalRealConsumer => {
            shared_published_producer_keeps_index_until_its_final_real_consumer();
            Ok(())
        }
        Scenario::TerminalShapeFailurePreservesActualReaderReleaseAndTransferredOutput => {
            terminal_shape_failure_preserves_actual_reader_release_and_transferred_output();
            Ok(())
        }
        Scenario::FinalRefusingConsumerReleasesZeroBudgetIndexWithoutRereadingItsProducer => {
            final_refusing_consumer_releases_zero_budget_index_without_rereading_its_producer();
            Ok(())
        }
        Scenario::UpfrontSkippedReaderReleasesTheRealSharedIndexAfterTheRemainingReader => {
            upfront_skipped_reader_releases_the_real_shared_index_after_the_remaining_reader();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
