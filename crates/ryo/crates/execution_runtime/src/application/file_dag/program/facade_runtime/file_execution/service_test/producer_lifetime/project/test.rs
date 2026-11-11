//! Real publication scopes retain and release project batch leases.
use super::fixtures::PhaseExit;
use super::fixtures::{Delivery, exercise};

#[test]
fn project_batch_success_releases_actual_phase_leases() {
    exercise(PhaseExit::Complete, Delivery::Once).expect("successful project lifecycle");
}

#[test]
fn project_batch_semantic_failure_releases_actual_phase_leases() {
    exercise(PhaseExit::SemanticFailure, Delivery::Once).expect("failed project lifecycle");
}

#[test]
fn project_batch_refusal_before_work_releases_actual_phase_leases() {
    exercise(PhaseExit::RefuseBeforeWork, Delivery::Once).expect("project refusal before work");
}

#[test]
fn project_batch_refusal_before_later_cohort_releases_actual_phase_leases() {
    exercise(PhaseExit::RefuseLaterCohort, Delivery::Once)
        .expect("project refusal before later cohort");
}

#[test]
fn project_duplicate_delivery_preserves_terminal_failure_and_cancelled_cohort_backing() {
    for exit in [
        PhaseExit::Complete,
        PhaseExit::SemanticFailure,
        PhaseExit::RefuseLaterCohort,
    ] {
        exercise(exit, Delivery::Replayed).unwrap_or_else(|error| panic!("{exit:?}: {error:?}"));
    }
}
