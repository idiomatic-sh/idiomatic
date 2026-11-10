//! Actual compiled request and services for the project lifecycle proof.
use super::super::super::*;
use super::super::fixtures::{FinalConsumer, ProjectionOrder, shared_plan};
use crate::application::file_dag::program::preparation::types::PreparedProgram;

#[derive(Clone, Copy, Debug)]
pub(super) enum PhaseExit {
    Complete,
    SemanticFailure,
    RefuseBeforeWork,
    RefuseLaterCohort,
}

pub(super) struct PreparedCase {
    pub(super) prepared: Arc<PreparedProgram>,
    pub(super) services: Arc<ProgramServices>,
    pub(super) corpus: [CorpusInput; 1],
}

pub(super) fn prepare_case(exit: PhaseExit) -> Result<PreparedCase, OperatingError> {
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
    let derived = ryo_execution_store::application::file_dag::fixtures::program_derived_port();
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
    let corpus = [CorpusInput::Available(CorpusEntry {
        executable: ExecutablePermission::Unknown,
        path: FilePath("project-roots.input".into()),
        size: FileSize(0),
        modified: Nanoseconds(0),
    })];
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(Arc::new(
            crate::application::file_dag::fixtures::operators(),
        ))),
        derived,
        &corpus,
    ));
    Ok(PreparedCase {
        prepared,
        services,
        corpus,
    })
}
