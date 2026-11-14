//! Actual compiled request and services for the project lifecycle proof.
use crate::application::file_dag::program::facade_runtime::service::ProgramServices;
use ryo_execution_domain::domain::operating::errors::OperatingError;
pub(super) mod scenario;
use super::super::super::fixtures::*;
use super::super::fixtures::{FinalConsumer, ProjectionOrder, shared_plan};
use crate::application::file_dag::execution::service::CommonOperatorEngine;
use crate::application::file_dag::frontiers::fixtures::prepared_frontiers;
use crate::application::file_dag::frontiers::types::FrontierInput;
use crate::application::file_dag::frontiers::types::FrontierSource;
use crate::application::file_dag::program::facade_runtime::project_execution::service::ProjectRoot;
use crate::application::file_dag::program::facade_runtime::project_execution::service::evaluate_roots;
use crate::application::file_dag::program::facade_runtime::service::ProjectFrontiers;
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::application::file_dag::program::rule_outcome::types::ProjectBatch;
use ryo_execution_domain::domain::{
    completion::types::CoverageState,
    coverage::types::Partials,
    table_model::types::Frontier,
};
use ryo_execution_domain::domain::{
    observation_schedule::types::RuleSchedule,
    prepared_request::types::{Compiled, ObservationIdentity},
};
use ryo_execution_store::application::file_dag::derived::owner::witness::service::BackingLiveness;
use ryo_execution_store::application::file_dag::derived::relation_owner::types::ScopePartials;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Relation;
use ryo_relations::domain::relations::{
    service::kinds,
    types::{FileId, Integer, Value as Cell},
};
use ryo_values::domain::rows::types::{RowCount, Table};
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{Input as PlanInput, StepId, StepScope},
    runs::types::RuleId,
    stored_facts::types::Digest,
};
use std::rc::Rc;
use std::sync::Arc;

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

#[derive(Clone, Copy, Debug)]
pub(super) enum Delivery {
    Once,
    Replayed,
}
