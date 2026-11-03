//! Adapter from the engine port to the portable projection use case.
use crate::outbound::operators::interface::OperatorsPort;
use ryo_operators::domain::contract::types::{OperatorAnswer, OperatorCall, PortableOperatorFault};
use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;
use ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl;

/// Constructed at a composition edge, with no per-row allocation.
#[derive(Debug)]
pub(crate) struct OperatorsUseCaseAdapter {
    operators: OperatorsUseCaseImpl,
}

impl OperatorsUseCaseAdapter {
    pub(crate) const fn new() -> Self {
        Self {
            operators: OperatorsUseCaseImpl::new(),
        }
    }
}

impl OperatorsPort for OperatorsUseCaseAdapter {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        self.operators.operators(call)
    }
}
