//! The memory fakes of the `budget` group's scenarios.

use crate::application::cross_file::fixtures::project_calls::ProjectCalls;
use crate::outbound::memory::errors::MemoryError;
use crate::outbound::memory::interface::MemoryPort;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_values::domain::runs::types::MemoryBytes;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Default)]
pub(super) struct GrowingMemory {
    pub(super) actual: ProjectCalls,
    pub(super) resident: AtomicU64,
}

impl ryo_execution_runtime::outbound::operators::interface::OperatorsPort for GrowingMemory {
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
        ryo_operators::inbound::use_cases::run::fixtures::ran(
            plan, node, buffers, inputs, resources,
        )
    }
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        let answer = self.actual.operators(call);
        self.resident.store(200, Ordering::SeqCst);
        answer
    }
}

impl MemoryPort for GrowingMemory {
    fn budget(&self) -> Result<MemoryBytes, MemoryError> {
        Ok(MemoryBytes(100))
    }
    fn resident_bytes(&self) -> Result<MemoryBytes, MemoryError> {
        Ok(MemoryBytes(self.resident.load(Ordering::SeqCst)))
    }
}

/// Resident bytes read in turn from a script, then none: each read is the next reading.
#[derive(Debug)]
pub(super) struct ScriptedMemory {
    pub(super) readings: &'static [u64],
    pub(super) read: std::sync::atomic::AtomicUsize,
}

impl MemoryPort for ScriptedMemory {
    fn budget(&self) -> Result<MemoryBytes, MemoryError> {
        Ok(MemoryBytes(100))
    }
    fn resident_bytes(&self) -> Result<MemoryBytes, MemoryError> {
        let at = self.read.fetch_add(1, Ordering::SeqCst);
        Ok(MemoryBytes(self.readings.get(at).copied().unwrap_or(0)))
    }
}
