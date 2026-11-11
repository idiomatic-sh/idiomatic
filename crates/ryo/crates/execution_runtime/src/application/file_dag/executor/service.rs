//! The `executor` application operation.

use super::types::{KernelAnswer, KernelCall, KernelFault};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;

pub(crate) trait PrimitiveExecutor: std::fmt::Debug + Sync {
    fn execute(&self, call: KernelCall<'_, '_, '_>) -> Result<KernelAnswer, KernelFault>;
}
