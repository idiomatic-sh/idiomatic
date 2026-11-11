//! The types of the `project_execution` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use super::super::super::project_population::types::ProjectFrontierPopulation;
use super::service::ProjectRun;
use crate::application::file_dag::program::admission::service::AvailableInput;
use crate::application::file_dag::program::admission::types::Partition;
use crate::application::file_dag::program::admission::types::RequiredPartitions;
use crate::application::file_dag::program::facade_runtime::service::ProgramServices;
use crate::application::file_dag::program::facade_runtime::service::ProjectFrontiers;
use crate::application::file_dag::program::facade_runtime::service::PublicationScope;
use crate::application::file_dag::program::native_walk::types::Deliveries;
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::application::file_dag::program::rule_outcome::types::ProjectOutputOutcome;
use crate::application::file_dag::program::rule_outcome::types::RuleOutcome;
use crate::application::file_dag::program::runtime::service::ProgramExecution;
use rustc_hash::FxHashMap;
use ryo_execution_domain::domain::completion::types::completion_policy::value::CompletionPolicy;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::rule_intent::types::EmitDemand;
use ryo_execution_domain::domain::rule_intent::types::ObservationDemand;
use ryo_execution_domain::domain::rule_intent::types::RuleDemand;
use ryo_execution_store::application::file_dag::derived::population::service::PopulationId;
use ryo_execution_store::application::file_dag::relations::service::RelationOwner;
use ryo_execution_store::application::file_dag::service::ScopePartials;
use std::rc::Rc;
use std::sync::Arc;

/// Cohort binding never reads source geometry. Exact immutable source/policy identity
/// selects a shared state; the existing interpreter remains the only evaluator.
/// A declared root travels with its own run, so a submission cannot omit or add a
/// run without also omitting or adding its declaration.
pub(crate) struct ProjectRoot<'a> {
    pub(crate) rule: RuleIndex,
    pub(crate) demand: RuleDemand,
    pub(crate) run: Result<ProjectRun<'a>, OperatingError>,
}
