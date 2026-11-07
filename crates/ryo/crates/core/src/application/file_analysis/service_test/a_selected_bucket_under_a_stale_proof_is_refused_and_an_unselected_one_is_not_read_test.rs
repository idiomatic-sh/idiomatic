//! The scenario `a_selected_bucket_under_a_stale_proof_is_refused_and_an_unselected_one_is_not_read` of the cases of this file.

use super::*;

pub(super) fn a_selected_bucket_under_a_stale_proof_is_refused_and_an_unselected_one_is_not_read() -> Result<Value, Failure> {
    use ryo_values::domain::records::types::FrontierCoverage;
    use ryo_values::domain::stored_facts::types::{Bucket, Digest};
    let good = crate::outbound::lint_store::fixtures::proof();
    let selected = BucketsWanted::These(vec![Bucket(0)]);
    replay_under(good.clone(), &selected).expect("the proof of the prepared root is accepted");
    let mut wrong_digest = good.clone();
    wrong_digest.rule_digest = Digest([9; 16]);
    let mut wrong_ordinal = good.clone();
    wrong_ordinal.input = ryo_values::domain::records::types::FrontierOrdinal(1);
    let mut wrong_coverage = good.clone();
    wrong_coverage.coverage = FrontierCoverage::Partial(Vec::new());
    let refused = |result: Result<(), FileAnalysisError>| {
        matches!(
            result,
            Err(FileAnalysisError::Execution(
                ryo_execution::inbound::use_cases::execution::types::OperatingError::ObservationIncomplete { .. }
            ))
        )
    };
    assert!(
        refused(replay_under_capture(good.clone(), &selected, false)),
        "a bucket whose frontier the prepared root does not capture is refused when selected"
    );
    replay_under_capture(good.clone(), &BucketsWanted::These(Vec::new()), false)
        .expect("the capture flag of a bucket the run does not select is never checked");
    for stale in [wrong_digest, wrong_ordinal, wrong_coverage] {
        assert!(
            matches!(
                replay_under(stale.clone(), &selected),
                Err(FileAnalysisError::Execution(
                    ryo_execution::inbound::use_cases::execution::types::OperatingError::ObservationIncomplete { .. }
                ))
            ),
            "{stale:?} must be refused for a selected bucket"
        );
        replay_under(stale, &BucketsWanted::These(Vec::new()))
            .expect("the proof of a bucket the run does not select is never checked");
    }
    Ok(1)
}
