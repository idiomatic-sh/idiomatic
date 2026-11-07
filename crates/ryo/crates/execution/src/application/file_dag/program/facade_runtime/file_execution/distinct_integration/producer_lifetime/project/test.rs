//! Real publication scopes retain and release project batch leases.
use super::super::super::*;
use super::fixtures::{PhaseExit, PreparedCase, prepare_case};
use crate::application::file_dag::derived::owner::witness::service::BackingLiveness;
use crate::application::file_dag::frontiers::types::FrontierInput;
use crate::application::file_dag::frontiers::types::FrontierSource;
use crate::application::file_dag::frontiers::types::PreparedFrontiers;
use crate::application::file_dag::program::facade_runtime::project_execution::ProjectRoot;
use crate::application::file_dag::program::facade_runtime::project_execution::evaluate_roots;
use crate::application::file_dag::program::facade_runtime::types::ProjectFrontiers;
use crate::application::file_dag::program::protocol::types::ProjectBatch;
use crate::domain::{
    completion::types::CoverageState, coverage::types::Partials, table_model::types::Frontier,
};
use ryo_values::domain::rows::errors::RowsError;

#[derive(Clone, Copy)]
enum Delivery {
    Once,
    Replayed,
}

fn exercise(exit: PhaseExit, delivery: Delivery) -> Result<(), OperatingError> {
    let PreparedCase {
        prepared,
        services,
        corpus,
    } = prepare_case(exit)?;
    for (rule, program) in prepared.program().rules().iter().enumerate() {
        for (step, origin) in program.original_steps.iter().enumerate() {
            let origin = origin.as_ref().unwrap_or_else(|error| {
                panic!("{exit:?}: compiled rule {rule} step {step}: {error:?}")
            });
            let region = prepared.program().logical_regions()[origin.logical.0];
            assert!(
                prepared.program().regions()[region.0]
                    .instructions()
                    .iter()
                    .any(|instruction| { instruction.logical() == origin.logical }),
                "{exit:?}: compiled rule {rule} step {step} must retain its actual instruction"
            );
        }
    }
    let phase = crate::application::file_dag::program::facade_runtime::service::publication_owner(
        &prepared, &services,
    )?;
    let owner = Rc::clone(phase.owner());
    // A project phase has no file-cache backend. Required derived views still
    // use this real owner; optional retention requires an installed file cache.
    // Configuring its absent backend would refuse before any root is submitted.
    assert!(
        !owner.derived().index_observation().available,
        "project phase must not install a file-only optional cache for this proof"
    );
    assert_eq!(owner.derived().retained_bytes(), 0);
    let required =
        crate::application::file_dag::program::facade_runtime::service::required_partitions(
            &prepared,
            RuleIndex(0),
            &corpus,
            &[],
            &[],
            &[],
            &services,
        )?;
    let source = PlanInput::Relation(Relation::LanguageTarget);
    let source_root = prepared.program().rules()[0].frontier[0].clone();
    let partials = Partials::default();
    let mut symbols = Symbols::default();
    let mut runs = Vec::new();
    for at in 0..4 {
        let distinct = at == 3 && matches!(exit, PhaseExit::RefuseLaterCohort);
        let table = Table {
            kinds: kinds(Relation::LanguageTarget).to_vec(),
            cells: vec![
                Cell::File(FileId(0)),
                Cell::Number(Integer(if distinct { 8 } else { 2 })),
                Cell::Number(Integer(9)),
                Cell::File(FileId(0)),
                Cell::Number(Integer(3)),
                Cell::Number(Integer(7)),
            ],
            count: RowCount(2),
        };
        runs.push(
            crate::application::file_dag::program::facade_runtime::project_execution::open_project(
                &prepared,
                required.clone(),
                ScopePartials::Project {
                    evaluation: &partials,
                    publication: &partials,
                },
                ProjectFrontiers {
                    publication: &phase,
                    ordinary: PreparedFrontiers {
                        producer: RuleIndex(at),
                        sources: vec![FrontierSource {
                            input: source,
                            root: source_root.clone(),
                        }]
                        .into(),
                        rows: vec![FrontierInput::Decoded {
                            frontier: Frontier {
                                input: source,
                                table: std::sync::Arc::new(table),
                            },
                            coverage: CoverageState::Complete,
                        }]
                        .into(),
                        program: Arc::clone(prepared.program()),
                        required: required.clone(),
                        refused: vec![].into(),
                    },
                    bound_inputs: &[],
                },
                &mut symbols,
                &[],
                Arc::clone(&services),
            ),
        );
    }
    assert!(
        runs.iter().all(Result::is_ok),
        "all submitted project owners bind before scheduling"
    );
    let demand = RuleDemand {
        emit: EmitDemand::Enabled,
        observations: ObservationDemand::NotRequired,
    };
    let roots: Vec<_> = runs
        .into_iter()
        .enumerate()
        .map(|(at, run)| ProjectRoot {
            rule: RuleIndex(at),
            demand,
            run,
        })
        .collect();
    let guards = std::cell::Cell::new(0);
    let result = evaluate_roots(roots, &mut symbols, |rule| {
        guards.set(guards.get() + 1);
        if matches!(exit, PhaseExit::RefuseBeforeWork)
            || (matches!(exit, PhaseExit::RefuseLaterCohort) && rule == RuleIndex(3))
        {
            Err(OperatingError::WrongBinding)
        } else {
            Ok(())
        }
    });
    let completed = match (exit, result) {
        (PhaseExit::RefuseBeforeWork, ProjectBatch::Refused { completed, error }) => {
            assert!(matches!(error, OperatingError::WrongBinding));
            assert!(completed.is_empty());
            assert_eq!(
                phase.readiness_counts().evaluations.0,
                0,
                "guard refusal cannot admit any root"
            );
            assert_eq!(owner.derived().working_bytes(), 0);
            assert_eq!(owner.derived().counts().0, 0);
            completed
        }
        (PhaseExit::RefuseLaterCohort, ProjectBatch::Refused { completed, error }) => {
            assert!(matches!(error, OperatingError::WrongBinding));
            assert_eq!(
                completed.len(),
                3,
                "earlier terminal deliveries survive batch cancellation"
            );
            assert_eq!(
                guards.get(),
                2,
                "later population guard runs before its scheduling"
            );
            completed
        }
        (PhaseExit::Complete | PhaseExit::SemanticFailure, ProjectBatch::Complete(completed)) => {
            assert_eq!(completed.len(), 4);
            assert_eq!(
                guards.get(),
                1,
                "equal populations use one actual batch admission"
            );
            completed
        }
        _ => panic!("unexpected project batch disposition for {exit:?}"),
    };
    let counts = phase.readiness_counts();
    let builds = owner.derived().counts();
    let live = owner.derived().working_bytes();
    assert_eq!(
        owner.derived().retained_bytes(),
        0,
        "project acquisition cannot retain optional views without a file backend"
    );
    assert!(
        !owner.derived().index_observation().available,
        "actual project execution must preserve its backend-free ownership boundary"
    );
    let backing = owner
        .derived()
        .view_witness(ryo_derived::domain::contract::types::DerivedKind::Equality);
    if !matches!(exit, PhaseExit::RefuseBeforeWork) {
        assert!(
            live > 0,
            "phase owns required backing until every batch obligation ends"
        );
        assert_eq!(builds.0, 1, "refused later cohort creates no second index");
        assert!(backing.as_ref().unwrap().liveness() == BackingLiveness::Alive);
        assert!(counts.evaluations.0 > 0);
    }
    let mut transferred = Vec::new();
    for (at, result) in completed.into_iter().enumerate() {
        let result = result.unwrap_or_else(|error| {
            panic!(
                "{exit:?}: project batch result for rule {at}: {error:?}; \
                readiness={counts:?}; resources={builds:?}; required={live:?}"
            )
        });
        match result.emit {
            RootOutcome::Complete(table) => transferred.push(table),
            RootOutcome::Failed(error) if at == 3 && matches!(exit, PhaseExit::SemanticFailure) => {
                assert!(matches!(
                    error,
                    OperatingError::Table(RowsError::NoSuchColumn { index: 99 })
                ));
            }
            _ => panic!("completed project root lost terminal result"),
        }
    }
    if transferred.len() >= 2 {
        assert_eq!(
            transferred[0], transferred[1],
            "deduplicated project root delivers both consumers"
        );
        assert_eq!(transferred[0].count.0, 2);
    }
    if matches!(delivery, Delivery::Replayed) && !matches!(exit, PhaseExit::RefuseBeforeWork) {
        // A new public delivery object must bind the same actual population;
        // it cannot revive terminal roots or consume another alias's obligation.
        for at in [0, 1, 3] {
            if at == 3 && matches!(exit, PhaseExit::RefuseLaterCohort) {
                continue;
            }
            let mut replay = crate::application::file_dag::program::facade_runtime::project_execution::open_project(
                &prepared,
                required.clone(),
                ScopePartials::Project {
                    evaluation: &partials,
                    publication: &partials,
                },
                ProjectFrontiers {
                    publication: &phase,
                    ordinary: PreparedFrontiers {
                        producer: RuleIndex(at),
                        sources: vec![FrontierSource {
                            input: source,
                            root: source_root.clone(),
                        }]
                        .into(),
                        rows: vec![FrontierInput::Decoded {
                            frontier: Frontier {
                                input: source,
                                table: Arc::new(Table {
                                    kinds: kinds(Relation::LanguageTarget).to_vec(),
                                    cells: vec![
                                        Cell::File(FileId(0)),
                                        Cell::Number(Integer(2)),
                                        Cell::Number(Integer(9)),
                                        Cell::File(FileId(0)),
                                        Cell::Number(Integer(3)),
                                        Cell::Number(Integer(7)),
                                    ],
                                    count: RowCount(2),
                                }),
                            },
                            coverage: CoverageState::Complete,
                        }].into(),
                        program: Arc::clone(prepared.program()),
                        required: required.clone(),
                        refused: vec![].into(),
                    },
                    bound_inputs: &[],
                },
                &mut symbols,
                &[],
                Arc::clone(&services),
            )?;
            for _ in 0..2 {
                let outcome = replay.rule(RuleIndex(at), demand, &mut symbols)?;
                if at == 3 && matches!(exit, PhaseExit::SemanticFailure) {
                    assert!(
                        matches!(
                            outcome.emit,
                            RootOutcome::Failed(OperatingError::Table(RowsError::NoSuchColumn {
                                index: 99
                            }))
                        ),
                        "outer project delivery preserves the original terminal error"
                    );
                } else {
                    let RootOutcome::Complete(table) = outcome.emit else {
                        panic!("terminal project replay")
                    };
                    assert!(table.count.0 > 0);
                }
                assert_eq!(
                    phase.readiness_counts(),
                    counts,
                    "project replay cannot schedule terminal roots"
                );
                assert_eq!(owner.derived().counts(), builds);
                assert_eq!(owner.derived().working_bytes(), live);
                assert!(
                    backing.as_ref().unwrap().liveness() == BackingLiveness::Alive,
                    "duplicate outer deliveries retain the same phase lease"
                );
            }
        }
    }
    phase.finish().unwrap_or_else(|error| {
        panic!("{exit:?}: phase finish after every terminal result: {error:?}")
    });
    assert_eq!(
        owner.derived().working_bytes(),
        0,
        "success/error/cancellation all release phase charges"
    );
    assert_eq!(owner.derived().owned_bytes(), 0);
    assert!(
        backing.is_none_or(|backing| backing.liveness() == BackingLiveness::Released),
        "same real backing drops at phase finish"
    );
    assert_eq!(
        owner.derived().counts(),
        builds,
        "finish cannot dispatch or acquire work"
    );
    for table in transferred {
        assert!(table.count.0 > 0, "transferred rows survive retirement");
    }
    Ok(())
}

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
