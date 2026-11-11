//! The helpers of the `interpreter_budget` group's scenarios.

use crate::application::cross_file::fixtures::ProjectCalls;
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
    resident: AtomicU64,
}

impl ryo_execution_runtime::outbound::operators::interface::OperatorsPort for GrowingMemory {
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
