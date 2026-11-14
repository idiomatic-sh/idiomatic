//! The planner's one dispatch over the operation kinds and the runner of a plan's kernels.
use crate::domain::contract::types::{Occurrences, PortableOperatorFault};
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{PlanInputs, PlanNodeId, Resources};

/// The physical plan of `program`.
///
/// # Errors
///
/// `WrongOperation` when a node reads a later node.
pub(crate) fn plan(program: &LogicalProgram<'_>) -> Result<PhysicalPlan, PortableOperatorFault> {
    program.check_shape()?;
    Ok(PhysicalPlan::default())
}

/// Makes `node`'s rows for the open file in `buffers`.
///
/// # Errors
///
/// `NotRun` until the kernels land: no node is lowered.
pub(crate) fn run(
    _plan: &PhysicalPlan,
    _node: PlanNodeId,
    _buffers: &mut FileBuffers,
    _inputs: &dyn PlanInputs,
    _resources: &mut dyn Resources,
    _occurrences: &dyn Occurrences,
) -> Result<(), PortableOperatorFault> {
    Err(PortableOperatorFault::NotRun)
}
