//! Adapter from the engine port to the portable projection use case.
use crate::outbound::operators::interface::OperatorsPort;
use ryo_operators::domain::contract::types::{OperatorAnswer, OperatorCall, PortableOperatorFault};
use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;

/// Constructed at a composition edge, with no per-row allocation.
#[derive(Debug)]
pub(crate) struct OperatorsUseCaseAdapter<T> {
    operators: T,
}

impl<T> OperatorsUseCaseAdapter<T> {
    pub(crate) const fn new(operators: T) -> Self {
        Self { operators }
    }
}

impl<T: OperatorsUseCase + Send + Sync + std::fmt::Debug> OperatorsPort
    for OperatorsUseCaseAdapter<T>
{
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        self.operators.operators(call)
    }
}
