//! One source pass and one frontier decode per population across the rules of a cohort, and
//! retirement on every exit.
use super::super::super::super::super::*;
use super::super::fixtures::{PhaseExit, PreparedCase, prepare_case};
use super::fixtures::{Stored, exercise};
use crate::application::file_dag::frontiers::types::DecodeCount;
use crate::application::file_dag::program::native_walk::project::service::ProjectWalk;
use crate::application::file_dag::program::native_walk::{Deliveries, RetainedBatches};

#[test]
fn one_population_is_read_once_for_every_rule_of_its_cohort() {
    let seen = exercise(PhaseExit::Complete, [2, 2, 2, 2], Stored::Decoded);
    assert_eq!(seen.delivered, 4, "every rule of the cohort is delivered");
    assert_eq!(seen.counts.passes.0, 1, "one source pass serves four rules");
    assert_eq!(
        seen.counts.source_rows.0, 2,
        "the population's rows are walked once"
    );
    assert_eq!(
        seen.retained,
        RetainedBatches(0),
        "the last consumer retired the batch"
    );
}

#[test]
fn a_cohort_of_a_second_population_reads_its_own_source() {
    let seen = exercise(PhaseExit::Complete, [2, 2, 8, 8], Stored::Decoded);
    assert_eq!(seen.delivered, 4);
    assert_eq!(
        seen.counts.passes.0, 2,
        "two populations are two source passes"
    );
    assert_eq!(
        seen.counts.source_rows.0, 4,
        "each population's rows are walked once, never shared with the other"
    );
    assert_eq!(seen.retained, RetainedBatches(0));
}

#[test]
fn retirement_follows_a_failing_terminal_result() {
    let seen = exercise(PhaseExit::SemanticFailure, [2, 2, 2, 2], Stored::Decoded);
    assert_eq!(seen.delivered, 4);
    assert_eq!(
        seen.counts.passes.0, 1,
        "an error in one rule does not reread the source"
    );
    assert_eq!(
        seen.retained,
        RetainedBatches(0),
        "the error left no batch behind"
    );
}

#[test]
fn retirement_follows_a_cancelled_batch() {
    let seen = exercise(PhaseExit::RefuseLaterCohort, [2, 2, 8, 8], Stored::Decoded);
    assert!(seen.refused, "the guard refused the later cohort");
    assert_eq!(
        seen.delivered, 2,
        "the two rules delivered before the refused cohort keep their terminal deliveries"
    );
    assert_eq!(
        seen.counts.passes.0, 1,
        "the refused cohort never reads its source"
    );
    assert_eq!(
        seen.retained,
        RetainedBatches(0),
        "cancellation retires the earlier cohort's batch and leaves the refused one unread"
    );
}

#[test]
fn one_population_is_decoded_once_for_every_rule_of_its_cohort() {
    let seen = exercise(PhaseExit::Complete, [2, 2, 2, 2], Stored::Packed);
    assert_eq!(seen.delivered, 4, "every rule of the cohort is delivered");
    assert_eq!(
        seen.decodes,
        DecodeCount(1),
        "four rules reading their own copies of one population's words decode them once"
    );
    assert_eq!(
        seen.counts.passes.0, 1,
        "the shared decode is one population, read in one pass"
    );
    assert_eq!(seen.retained, RetainedBatches(0));
}

#[test]
fn a_second_population_is_decoded_on_its_own() {
    let seen = exercise(PhaseExit::Complete, [2, 2, 8, 8], Stored::Packed);
    assert_eq!(seen.delivered, 4);
    assert_eq!(
        seen.decodes,
        DecodeCount(2),
        "two populations are two decodes, one for each, never one for each rule"
    );
    assert_eq!(
        seen.counts.passes.0, 2,
        "two populations are two source passes"
    );
    assert_eq!(seen.retained, RetainedBatches(0));
}

#[test]
fn decoded_rows_are_never_decoded_again() {
    let seen = exercise(PhaseExit::Complete, [2, 2, 2, 2], Stored::Decoded);
    assert_eq!(
        seen.decodes,
        DecodeCount(0),
        "rows handed decoded are admitted as they are"
    );
}

#[test]
fn a_head_is_admitted_only_for_a_declared_root() {
    let PreparedCase { prepared, .. } = prepare_case(PhaseExit::Complete).expect("compile");
    let class = prepared
        .walk()
        .class_of(RuleIndex(0))
        .and_then(|classes| classes.first().copied().flatten())
        .expect("the project reader's first step is a shared head");
    let walk = ProjectWalk::new();
    assert!(
        matches!(walk.admit(&prepared, class), Err(OperatingError::NotRead)),
        "a head whose rule declared no root is refused by name"
    );
    walk.owe(&prepared, RuleIndex(1), Deliveries(1));
    walk.admit(&prepared, class)
        .expect("a sibling that declared the same class shares it");
    walk.delivered(&prepared, RuleIndex(1));
    assert!(
        matches!(walk.admit(&prepared, class), Err(OperatingError::NotRead)),
        "after its last delivery the declaration is gone"
    );
}
