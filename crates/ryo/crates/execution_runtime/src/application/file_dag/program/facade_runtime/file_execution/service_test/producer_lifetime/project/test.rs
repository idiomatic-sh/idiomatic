//! Real publication scopes retain and release project batch leases.
use super::fixtures::Delivery;
use super::fixtures::PhaseExit;
use super::fixtures::scenario::exercise;
use crate::cases::{Case, CaseName, Expect, Word, run};

fn project_batch_success_releases_actual_phase_leases() {
    exercise(PhaseExit::Complete, Delivery::Once).expect("successful project lifecycle");
}

fn project_batch_semantic_failure_releases_actual_phase_leases() {
    exercise(PhaseExit::SemanticFailure, Delivery::Once).expect("failed project lifecycle");
}

fn project_batch_refusal_before_work_releases_actual_phase_leases() {
    exercise(PhaseExit::RefuseBeforeWork, Delivery::Once).expect("project refusal before work");
}

fn project_batch_refusal_before_later_cohort_releases_actual_phase_leases() {
    exercise(PhaseExit::RefuseLaterCohort, Delivery::Once)
        .expect("project refusal before later cohort");
}

fn project_duplicate_delivery_preserves_terminal_failure_and_cancelled_cohort_backing() {
    for exit in [
        PhaseExit::Complete,
        PhaseExit::SemanticFailure,
        PhaseExit::RefuseLaterCohort,
    ] {
        exercise(exit, Delivery::Replayed).unwrap_or_else(|error| panic!("{exit:?}: {error:?}"));
    }
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    ProjectBatchSuccessReleasesActualPhaseLeases,
    ProjectBatchSemanticFailureReleasesActualPhaseLeases,
    ProjectBatchRefusalBeforeWorkReleasesActualPhaseLeases,
    ProjectBatchRefusalBeforeLaterCohortReleasesActualPhaseLeases,
    ProjectDuplicateDeliveryPreservesTerminalFailureAndCancelledCohortBacking,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("project_batch_success_releases_actual_phase_leases"),
        word: Word::Succeeds,
        input: || Scenario::ProjectBatchSuccessReleasesActualPhaseLeases,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("project_batch_semantic_failure_releases_actual_phase_leases"),
        word: Word::Succeeds,
        input: || Scenario::ProjectBatchSemanticFailureReleasesActualPhaseLeases,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("project_batch_refusal_before_work_releases_actual_phase_leases"),
        word: Word::Succeeds,
        input: || Scenario::ProjectBatchRefusalBeforeWorkReleasesActualPhaseLeases,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("project_batch_refusal_before_later_cohort_releases_actual_phase_leases"),
        word: Word::Succeeds,
        input: || Scenario::ProjectBatchRefusalBeforeLaterCohortReleasesActualPhaseLeases,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "project_duplicate_delivery_preserves_terminal_failure_and_cancelled_cohort_backing",
        ),
        word: Word::Succeeds,
        input: || {
            Scenario::ProjectDuplicateDeliveryPreservesTerminalFailureAndCancelledCohortBacking
        },
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::ProjectBatchSuccessReleasesActualPhaseLeases => {
            project_batch_success_releases_actual_phase_leases();
            Ok(())
        }
        Scenario::ProjectBatchSemanticFailureReleasesActualPhaseLeases => {
            project_batch_semantic_failure_releases_actual_phase_leases();
            Ok(())
        }
        Scenario::ProjectBatchRefusalBeforeWorkReleasesActualPhaseLeases => {
            project_batch_refusal_before_work_releases_actual_phase_leases();
            Ok(())
        }
        Scenario::ProjectBatchRefusalBeforeLaterCohortReleasesActualPhaseLeases => {
            project_batch_refusal_before_later_cohort_releases_actual_phase_leases();
            Ok(())
        }
        Scenario::ProjectDuplicateDeliveryPreservesTerminalFailureAndCancelledCohortBacking => {
            project_duplicate_delivery_preserves_terminal_failure_and_cancelled_cohort_backing();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
