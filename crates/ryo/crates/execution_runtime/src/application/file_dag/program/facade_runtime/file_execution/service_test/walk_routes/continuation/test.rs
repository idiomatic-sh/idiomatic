//! A source class is not a continuation result, and its consumers own immutable answers.
use super::super::fixtures::Residency;
use super::fixtures::exercise;

#[test]
fn shared_source_continuation_failure_and_result_consumers_preserve_one_certified_population() {
    exercise(Residency::Stable)
        .expect("actual shared class, continuations and selected/replayed consumers");
}
#[test]
fn uncertified_resident_identity_keeps_equal_source_bytes_in_distinct_populations() {
    exercise(Residency::Unstable).expect("real unstable source barrier");
}
