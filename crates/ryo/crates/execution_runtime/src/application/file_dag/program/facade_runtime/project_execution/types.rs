//! The types of the `project_execution` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use super::service::ProjectRun;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::rule_intent::types::RuleDemand;

/// Cohort binding never reads source geometry. Exact immutable source/policy identity
/// selects a shared state; the existing interpreter remains the only evaluator.
/// A declared root travels with its own run, so a submission cannot omit or add a
/// run without also omitting or adding its declaration.
pub(crate) struct ProjectRoot<'a> {
    pub(crate) rule: RuleIndex,
    pub(crate) demand: RuleDemand,
    pub(crate) run: Result<ProjectRun<'a>, OperatingError>,
}
