//! A program run with no probe index shared: what the sharing cases and the shared-index bench row compare against.

use super::{planned, ran};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::Step;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{
    LogicalNode, Lowering, NodeBindings, NodeRows, PlanInputList, PlanInputs, PlanNodeId,
    ProbeIndex, Resources, RuleOrdinal,
};
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::rows::types::Table;

/// `plan` with every probe on an index of its own.
#[must_use]
pub fn unshared(mut plan: PhysicalPlan) -> PhysicalPlan {
    for node in &mut plan.nodes {
        if let Lowering::Probe { index, .. } = &mut node.lowering {
            *index = ProbeIndex::Own;
        }
    }
    plan
}

/// The rows each emitted step answers, in emit order, or the first fault: as `emitted` answers them, with every probe on an
/// index of its own.
///
/// # Errors
///
/// The fault `plan` or `run` answers.
pub fn emitted_unshared(
    steps: &[Step],
    emits: &[PlanNodeId],
    inputs: &dyn PlanInputs,
    resources: &mut dyn Resources,
    buffers: &mut FileBuffers,
) -> Result<Vec<Table>, PortableOperatorFault> {
    let mut program = LogicalProgram::new();
    for step in steps {
        let _id = program.node(LogicalNode {
            operation: &step.operation,
            inputs: PlanInputList(&step.inputs),
            bindings: NodeBindings(&[]),
            scope: StepScope::PerFile,
        });
    }
    for (rule, node) in (0..).map(RuleOrdinal).zip(emits) {
        program.emit(rule, *node);
    }
    let plan = unshared(planned(&program)?);
    emits
        .iter()
        .map(|node| {
            ran(&plan, *node, buffers, inputs, resources)?;
            match buffers.rows(*node) {
                NodeRows::Materialized(table) => Ok(table.clone()),
                NodeRows::Absent => Err(PortableOperatorFault::NotRun),
            }
        })
        .collect()
}
