//! The runner's chain of streamed stages: the rows of the stages that feed a node, pushed through each stage in turn, only the
//! node's own rows held.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{KernelRun, PlanNodeId};

/// Makes `tail`'s rows by pushing the rows of the chain of streamed nodes that feeds it through each stage in turn, holding
/// only `tail`'s rows and pending fault. A node of the chain already held in `buffers` ends the chain and is read as a source.
///
/// # Errors
///
/// The chain's first hard fault in the held plan's order.
pub(in crate::domain::physical_plan::service::run) fn streamed(
    plan: &PhysicalPlan,
    run: &mut KernelRun<'_>,
    tail: PlanNodeId,
    buffers: &mut FileBuffers,
) -> Result<(), PortableOperatorFault> {
    let node = plan
        .nodes
        .get(usize::try_from(tail.0).unwrap_or(usize::MAX))
        .ok_or(PortableOperatorFault::WrongOperation)?;
    // The contract half: the chain is the tail alone, whatever its sources' flow, each source made and held first.
    let _alone = node.flow;
    super::super::service::made(plan, run, tail, &node.lowering, buffers)
}
