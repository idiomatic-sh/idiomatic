//! Required derived storage follows real compiled producer consumers, including cancellation.
use super::fixtures::FinalConsumer;
use super::fixtures::exercise;

#[test]
fn shared_published_producer_keeps_index_until_its_final_real_consumer() {
    exercise(FinalConsumer::Completes).expect("actual compiled producer lifetime");
}
#[test]
fn terminal_shape_failure_preserves_actual_reader_release_and_transferred_output() {
    exercise(FinalConsumer::InvalidTerminal).expect("actual compiled failure cancellation");
}

#[test]
fn final_refusing_consumer_releases_zero_budget_index_without_rereading_its_producer() {
    for final_consumer in [FinalConsumer::RefusingProvider] {
        exercise(final_consumer).expect("actual compiled final consumer cancellation");
    }
}

#[test]
fn upfront_skipped_reader_releases_the_real_shared_index_after_the_remaining_reader() {
    exercise(FinalConsumer::SkippedAtStart)
        .expect("actual zero-budget final reader release before file finish");
}
