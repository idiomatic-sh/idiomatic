//! Distinct state survives repeated left rows and resets after a failed delivery.
use super::super::fixtures::Scenario;
use super::fixtures::run;

#[test]
fn reach_restarts_distinct_for_each_identical_left_row() {
    let actual = run(Scenario::ReachDuplicate).expect("compiled reach");
    assert_eq!(actual, vec![1, 2, 3, 1, 2, 3]);
}
#[test]
fn project_and_union_retry_replay_completed_distinct_after_sink_failure() {
    for scenario in [Scenario::ProjectRetry, Scenario::UnionRetry] {
        let actual = run(scenario).expect("compiled retry");
        assert_eq!(actual, vec![0, 2]);
    }
}
