//! Sparse cohort merging and atomic overlap refusal.

use super::value::NativeBatch;
use crate::domain::{
    shared_walk::{
        service_test::{evaluate_plans, plans},
        types::prepared_walk::value::PreparedWalk,
    },
    step_classes::types::ClassPlace,
};

#[test]
fn sparse_cohorts_merge_only_reached_results_and_reject_overlap_atomically() {
    let plans = plans();
    let bindings = vec![Vec::new(); plans.len()];
    let prepared = PreparedWalk::compile(&plans, &bindings);
    let mut accumulated = NativeBatch::for_program(&prepared);
    assert_eq!(accumulated.counts().result_slots.0, 0);
    accumulated
        .merge(evaluate_plans(&prepared, &bindings, &[ClassPlace(0)]))
        .unwrap();
    assert_eq!(accumulated.counts().scratch_classes.0, 1);
    assert_eq!(accumulated.counts().result_slots.0, 1);
    assert!(accumulated.class(ClassPlace(1)).is_none());
    accumulated
        .merge(evaluate_plans(&prepared, &bindings, &[ClassPlace(1)]))
        .unwrap();
    assert_eq!(accumulated.counts().scratch_classes.0, 2);
    assert_eq!(accumulated.counts().result_slots.0, 2);
    let before = accumulated.counts();
    // Class 2 is new but must not be committed when another entry overlaps.
    assert!(
        accumulated
            .merge(evaluate_plans(
                &prepared,
                &bindings,
                &[ClassPlace(1), ClassPlace(2)]
            ))
            .is_err()
    );
    assert!(accumulated.class(ClassPlace(2)).is_none());
    assert_eq!(accumulated.counts().result_slots, before.result_slots);
    assert_eq!(accumulated.counts().passes, before.passes);
    assert_eq!(
        accumulated.class(ClassPlace(0)).unwrap().positions.0,
        [1, 3]
    );
    assert_eq!(accumulated.class(ClassPlace(1)).unwrap().positions.0, [2]);
}
