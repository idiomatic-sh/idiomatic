//! Where a planned program's steps run: each rule's emitted step and named outputs in each file or after the files, and each
//! node's rows for the whole phase or one occurrence.

use crate::domain::coverage::types::RuleIndex;
use crate::domain::executable_program::types::ObservationReach;
use crate::domain::executable_program::types::executable_program::value::ExecutableProgram;
use crate::domain::operating::errors::OperatingError;
use ryo_operators::domain::physical_plan::types::PlanNodeId;
use ryo_values::domain::plan_parts::types::{StepId, StepScope};
use ryo_values::domain::plans::types::Plan;

/// The scope the step `step` of `plan` runs in; a step the plan does not hold runs in each file.
pub(in crate::domain::executable_program) fn scope_at(plan: &Plan, step: StepId) -> StepScope {
    plan.steps
        .get(usize::try_from(step.0).unwrap_or(usize::MAX))
        .map_or(StepScope::PerFile, |each| each.scope)
}

/// Where `rule`'s emitted step runs: in each file, or after the files.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::service::{assemble, logical};
/// use ryo_execution_domain::domain::executable_program::service::scopes::service::emit_scope;
/// use ryo_execution_domain::domain::executable_program::types::program_inputs::value::ProgramInputs;
/// let program = assemble(logical(&[], &mut ProgramInputs::default()).unwrap(), Default::default());
/// assert!(emit_scope(&program, ryo_execution_domain::domain::coverage::types::RuleIndex(0)).is_err());
/// ```
///
/// # Errors
///
/// `NotRun` for a rule the program does not have.
pub fn emit_scope(
    program: &ExecutableProgram,
    rule: RuleIndex,
) -> Result<StepScope, OperatingError> {
    program
        .scopes
        .get(rule.0)
        .map(|scopes| scopes.emit)
        .ok_or(OperatingError::NotRun)
}

/// Where `rule`'s named output `output` is made: in each file, or after the files.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::service::{assemble, logical};
/// use ryo_execution_domain::domain::executable_program::service::scopes::service::observation_scope;
/// use ryo_execution_domain::domain::executable_program::types::program_inputs::value::ProgramInputs;
/// use ryo_values::domain::records::types::ObservationIndex;
/// let program = assemble(logical(&[], &mut ProgramInputs::default()).unwrap(), Default::default());
/// let rule = ryo_execution_domain::domain::coverage::types::RuleIndex(0);
/// assert!(observation_scope(&program, rule, ObservationIndex(0)).is_err());
/// ```
///
/// # Errors
///
/// `NotRun` for a rule or an output the program does not have.
pub fn observation_scope(
    program: &ExecutableProgram,
    rule: RuleIndex,
    output: ryo_values::domain::records::types::ObservationIndex,
) -> Result<StepScope, OperatingError> {
    program
        .scopes
        .get(rule.0)
        .and_then(|scopes| {
            scopes
                .observations
                .get(usize::try_from(output.0).unwrap_or(usize::MAX))
        })
        .copied()
        .ok_or(OperatingError::NotRun)
}

/// Whether `node`'s rows answer one occurrence's bound observations, and so go at each occurrence's end, or the whole phase.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::service::{assemble, logical};
/// use ryo_execution_domain::domain::executable_program::service::scopes::service::observation_reach;
/// use ryo_execution_domain::domain::executable_program::types::program_inputs::value::ProgramInputs;
/// use ryo_operators::domain::physical_plan::types::PlanNodeId;
/// let program = assemble(logical(&[], &mut ProgramInputs::default()).unwrap(), Default::default());
/// assert!(observation_reach(&program, PlanNodeId(0)).is_err());
/// ```
///
/// # Errors
///
/// `NotRun` for a node the program does not have.
pub fn observation_reach(
    program: &ExecutableProgram,
    node: PlanNodeId,
) -> Result<ObservationReach, OperatingError> {
    program
        .reach
        .get(usize::try_from(node.0).unwrap_or(usize::MAX))
        .copied()
        .ok_or(OperatingError::NotRun)
}
