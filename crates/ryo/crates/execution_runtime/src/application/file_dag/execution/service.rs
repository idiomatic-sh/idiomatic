//! The `execution` application operation.

use super::super::executor::{service::PrimitiveExecutor, types::KernelCall};
use crate::application::file_dag::executor::{types::KernelAnswer, types::KernelFault};
use crate::outbound::operators::interface::OperatorsPort;
#[derive(Debug)]
pub(crate) struct CommonOperatorEngine {
    operators: std::sync::Arc<dyn OperatorsPort>,
}

impl CommonOperatorEngine {
    pub(crate) fn new(operators: std::sync::Arc<dyn OperatorsPort>) -> Self {
        Self { operators }
    }
}

impl PrimitiveExecutor for CommonOperatorEngine {
    fn execute(&self, call: KernelCall<'_, '_, '_>) -> Result<KernelAnswer, KernelFault> {
        match call {
            KernelCall::Registered { request } => {
                request.dispatch().map_err(KernelFault::Refused)?;
                Ok(KernelAnswer::Complete)
            }
            KernelCall::Walk { request } => request
                .dispatch()
                .map(KernelAnswer::Walk)
                .map_err(KernelFault::Refused),
            KernelCall::Physical { request } => request
                .dispatch(self.operators.as_ref())
                .map(KernelAnswer::Physical),
        }
    }
}
