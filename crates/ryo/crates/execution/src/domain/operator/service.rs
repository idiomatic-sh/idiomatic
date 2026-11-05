//! Pure operator plan workspace and terminal access.
use super::types::Pool;
use crate::domain::{
    operating::errors::OperatingError, operation_schema::types::InputShape,
    operator::contract::types::OperatorContract, table_model::types::};
use ryo_values::domain::plans::types::{Operation, OperationKind};

/// Read-only contract preparation. This surface cannot evaluate an operation.
pub(crate) struct PreparationRoute(OperationKind);
impl PreparationRoute {
    pub(crate) fn prepare(
        &self,
        operation: &Operation,
        inputs: &[InputShape<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        super::contract::registration::registered(self.0).prepare(operation, inputs, bindings)
    }
}
pub(crate) fn registered(kind: OperationKind) -> PreparationRoute {
    PreparationRoute(kind)
}

use ryo_values::domain::plans::{
    service::step_scope,
    types::{Plan, StepScope},
};
/// Mutable buffers sized for the supplied plans. Production uses one plan in a local rule workspace.
#[must_use]
pub(crate) fn pool_for(plans: &[Plan]) -> Pool {
    let steps = plans.iter().map(|plan| plan.steps.len()).max().unwrap_or(0);
    let frontier = plans
        .iter()
        .map(|plan| plan.frontier.len())
        .max()
        .unwrap_or(0);
    Pool {
        steps: vec![Table::default(); steps],
        frontier: vec![Table::default(); frontier],
        ..Pool::default()
    }
}

/// The rows the plan emits for the file just evaluated when the emitting step runs per file.
#[must_use]
pub(crate) fn emitted<'p>(plan: &Plan, pool: &'p mut Pool) -> Option<&'p mut Table> {
    match step_scope(plan, plan.emit.step) {
        Some(StepScope::PerFile) => pool
            .steps
            .get_mut(usize::try_from(plan.emit.step.0).unwrap_or(usize::MAX)),
        Some(StepScope::CrossFile) | None => None,
    }
}

use ryo_values::domain::rows::types::Table;

use ryo_values::domain::query_bindings::types::{ParameterBinding};
