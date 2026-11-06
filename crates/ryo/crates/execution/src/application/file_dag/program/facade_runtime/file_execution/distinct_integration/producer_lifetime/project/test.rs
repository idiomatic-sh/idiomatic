//! Real publication scopes retain and release project batch leases.
use super::super::super::*;
use super::super::fixtures::{FinalConsumer, ProjectionOrder, shared_plan};
use crate::application::file_dag::frontiers::types::FrontierInput;
use crate::application::file_dag::frontiers::types::FrontierSource;
use crate::application::file_dag::frontiers::types::PreparedFrontiers;
use crate::application::file_dag::program::facade_runtime::ProjectFrontiers;
use crate::application::file_dag::program::facade_runtime::ProjectRootDeclaration;
use crate::application::file_dag::program::facade_runtime::evaluate_roots;
use crate::application::file_dag::program::protocol::types::ProjectBatch;
use crate::domain::{
    completion::types::CoverageState, coverage::types::Partials, table_model::types::Frontier,
};
use ryo_values::domain::rows::errors::TableModelError;

#[derive(Clone, Copy, Debug)]
enum PhaseExit {
    Complete,
    SemanticFailure,
    RefuseBeforeWork,
    RefuseLaterCohort,
}

fn exercise(exit: PhaseExit, replay: bool) -> Result<(), OperatingError> {
    let mut external = shared_plan(ProjectionOrder::Forward, FinalConsumer::Completes);
    external.steps.truncate(2);
    external.emit.step = StepId(1);
    let mut plans = vec![
        external.clone(),
        external,
        shared_plan(ProjectionOrder::Forward, FinalConsumer::Completes),
        shared_plan(
            ProjectionOrder::Reversed,
            if matches!(exit, PhaseExit::SemanticFailure) {
                FinalConsumer::InvalidTerminal
            } else {
                FinalConsumer::Completes
            },
        ),
    ];
    for plan in &mut plans {
        for step in &mut plan.steps {
            step.scope = StepScope::CrossFile;
        }
        plan.frontier = vec![PlanInput::Relation(Relation::LanguageTarget)];
    }
    let mut rules = vec![rule(); 4];
    for (at, rule) in rules.iter_mut().enumerate() {
        rule.id = RuleId(format!("phase-reader-{at}").into());
    }
    let derived = crate::application::file_dag::testing::program_derived_port();
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]; 4],
            plans,
            bindings: vec![vec![]; 4],
        },
        &rules,
        &vec![Digest([0; 16]); 4],
        &[],
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )?;
    for (rule, program) in prepared.program().rules().iter().enumerate() {
        for (step, origin) in program.original_steps.iter().enumerate() {
            let origin = origin.as_ref().unwrap_or_else(|error| {
                panic!("{exit:?}: compiled rule {rule} step {step}: {error:?}")
            });
            let region = prepared.program().logical_regions()[origin.logical.0];
            assert!(
                prepared.program().regions()[region.0]
                    .instructions
                    .iter()
                    .any(|instruction| { instruction.logical == origin.logical }),
                "{exit:?}: compiled rule {rule} step {step} must retain its actual instruction"
            );
        }
    }
    let corpus = [CorpusInput::Available(CorpusEntry {
        executable: ExecutablePermission::Unknown,
        path: FilePath("project-roots.input".into()),
        size: FileSize(0),
        modified: Nanoseconds(0),
    })];
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(Arc::new(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
        ))),
        derived,
        &corpus,
    ));
    let phase = crate::application::file_dag::program::facade_runtime::publication_owner(
        &prepared, &services,
    )?;
    let owner = Rc::clone(phase.owner());
    // A project phase has no file-cache backend. Required derived views still
    // use this real owner; optional retention requires an installed file cache.
    // Configuring its absent backend would refuse before any root is submitted.
    assert!(
        !owner.derived.index_observation().available,
        "project phase must not install a file-only optional cache for this proof"
    );
    assert_eq!(owner.derived.retained_bytes(), 0);
    let required = crate::application::file_dag::program::facade_runtime::required_partitions(
        &prepared,
        RuleIndex(0),
        &corpus,
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
            crate::application::file_dag::program::facade_runtime::open_project(
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
                        .into_boxed_slice(),
                        rows: vec![FrontierInput::Decoded {
                            frontier: Frontier {
                                input: source,
                                table,
                            },
                            coverage: CoverageState::Complete,
                        }],
                        program: Arc::clone(prepared.program()),
                        required: required.clone(),
                        refused: vec![],
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
    let declarations: Vec<_> = (0..4)
        .map(|at| ProjectRootDeclaration {
            rule: RuleIndex(at),
            demand: RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::NotRequired,
            },
        })
        .collect();
    let guards = std::cell::Cell::new(0);
    let result = evaluate_roots(runs, &declarations, &mut symbols, |rule| {
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
            assert_eq!(owner.derived.working_bytes(), 0);
            assert_eq!(owner.derived.counts().0, 0);
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
    let builds = owner.derived.counts();
    let live = owner.derived.working_bytes();
    assert_eq!(
        owner.derived.retained_bytes(),
        0,
        "project acquisition cannot retain optional views without a file backend"
    );
    assert!(
        !owner.derived.index_observation().available,
        "actual project execution must preserve its backend-free ownership boundary"
    );
    let backing = owner
        .derived
        .view_weak(ryo_derived::domain::contract::types::DerivedKind::Equality);
    if !matches!(exit, PhaseExit::RefuseBeforeWork) {
        assert!(
            live > 0,
            "phase owns required backing until every batch obligation ends"
        );
        assert_eq!(builds.0, 1, "refused later cohort creates no second index");
        assert!(backing.as_ref().unwrap().upgrade().is_some());
        assert!(counts.evaluations.0 > 0);
    }
    let mut transferred = Vec::new();
    for (at, result) in completed.into_iter().enumerate() {
        let result = result.unwrap_or_else(|error| {
            panic!(
                "{exit:?}: project batch result for rule {at}: {error:?}; \
                readiness={counts:?}; resources={builds:?}; required={live}"
            )
        });
        match result.emit {
            RootOutcome::Complete(table) => transferred.push(table),
            RootOutcome::Failed(error) if at == 3 && matches!(exit, PhaseExit::SemanticFailure) => {
                assert!(matches!(
                    error,
                    OperatingError::Table(TableModelError::NoSuchColumn { index: 99 })
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
    if replay && !matches!(exit, PhaseExit::RefuseBeforeWork) {
        // A new public delivery object must bind the same actual population;
        // it cannot revive terminal roots or consume another alias's obligation.
        for at in [0, 1, 3] {
            if at == 3 && matches!(exit, PhaseExit::RefuseLaterCohort) {
                continue;
            }
            let mut replay = crate::application::file_dag::program::facade_runtime::open_project(
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
                        .into_boxed_slice(),
                        rows: vec![FrontierInput::Decoded {
                            frontier: Frontier {
                                input: source,
                                table: Table {
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
                                },
                            },
                            coverage: CoverageState::Complete,
                        }],
                        program: Arc::clone(prepared.program()),
                        required: required.clone(),
                        refused: vec![],
                    },
                    bound_inputs: &[],
                },
                &mut symbols,
                &[],
                Arc::clone(&services),
            )?;
            for _ in 0..2 {
                let outcome = replay.rule(RuleIndex(at), declarations[at].demand, &mut symbols)?;
                if at == 3 && matches!(exit, PhaseExit::SemanticFailure) {
                    assert!(
                        matches!(
                            outcome.emit,
                            RootOutcome::Failed(OperatingError::Table(
                                TableModelError::NoSuchColumn { index: 99 }
                            ))
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
                assert_eq!(owner.derived.counts(), builds);
                assert_eq!(owner.derived.working_bytes(), live);
                assert!(
                    backing.as_ref().unwrap().upgrade().is_some(),
                    "duplicate outer deliveries retain the same phase lease"
                );
            }
        }
    }
    phase.finish().unwrap_or_else(|error| {
        panic!("{exit:?}: phase finish after every terminal result: {error:?}")
    });
    assert_eq!(
        owner.derived.working_bytes(),
        0,
        "success/error/cancellation all release phase charges"
    );
    assert_eq!(owner.derived.owned_bytes(), 0);
    assert!(
        backing.is_none_or(|backing| backing.upgrade().is_none()),
        "same real backing drops at phase finish"
    );
    assert_eq!(
        owner.derived.counts(),
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
    exercise(PhaseExit::Complete, false).expect("successful project lifecycle");
}

#[test]
fn project_batch_semantic_failure_releases_actual_phase_leases() {
    exercise(PhaseExit::SemanticFailure, false).expect("failed project lifecycle");
}

#[test]
fn project_batch_refusal_before_work_releases_actual_phase_leases() {
    exercise(PhaseExit::RefuseBeforeWork, false).expect("project refusal before work");
}

#[test]
fn project_batch_refusal_before_later_cohort_releases_actual_phase_leases() {
    exercise(PhaseExit::RefuseLaterCohort, false).expect("project refusal before later cohort");
}

#[test]
fn project_duplicate_delivery_preserves_terminal_failure_and_cancelled_cohort_backing() {
    for exit in [
        PhaseExit::Complete,
        PhaseExit::SemanticFailure,
        PhaseExit::RefuseLaterCohort,
    ] {
        exercise(exit, true).unwrap_or_else(|error| panic!("{exit:?}: {error:?}"));
    }
}
