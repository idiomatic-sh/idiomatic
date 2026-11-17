//! Controlled stand-in for the operator port; records and can refuse calls.
use crate::outbound::operators::interface::OperatorsPort;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Default)]
pub struct OperatorsFake {
    /// Number of nodes the engine ran through the fake.
    pub calls: AtomicUsize,
    /// Refuse each run before the portable implementation runs.
    pub refuses: bool,
}
impl OperatorsPort for OperatorsFake {
    fn plan(
        &self,
        program: &ryo_operators::domain::physical_plan::types::logical_program::value::LogicalProgram<'_>,
    ) -> Result<
        ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan,
        ryo_operators::domain::contract::types::PortableOperatorFault,
    > {
        ryo_operators::inbound::use_cases::run::fixtures::planned(program)
    }
    fn run(
        &self,
        plan: &ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan,
        node: ryo_operators::domain::physical_plan::types::PlanNodeId,
        buffers: &mut ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers,
        inputs: &dyn ryo_operators::domain::physical_plan::types::PlanInputs,
        resources: &mut dyn ryo_operators::domain::physical_plan::types::Resources,
    ) -> Result<(), ryo_operators::domain::contract::types::PortableOperatorFault> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        if self.refuses {
            return Err(PortableOperatorFault::NoSuchColumn {
                index: ryo_values::domain::plan_parts::types::ColumnIndex(8),
            });
        }
        ryo_operators::inbound::use_cases::run::fixtures::ran(
            plan, node, buffers, inputs, resources,
        )
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
