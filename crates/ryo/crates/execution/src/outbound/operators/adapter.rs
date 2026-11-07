//! The operators capability of another crate, held and delegated to behind the port.
use crate::outbound::operators::interface::OperatorsPort;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;
use ryo_operators::inbound::use_cases::operators::types::{OperatorAnswer, OperatorCall};

/// The operators port over the operators use case it holds.
#[derive(Debug)]
pub struct OperatorsAdapter<U> {
    operators: U,
}

impl<U> OperatorsAdapter<U> {
    /// The adapter over `operators`.
    #[must_use]
    pub const fn new(operators: U) -> Self {
        Self { operators }
    }
}

impl<U: OperatorsUseCase + Send + Sync + std::fmt::Debug> OperatorsUseCase for OperatorsAdapter<U> {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        self.operators.operators(call)
    }
}

impl<U: OperatorsUseCase + Send + Sync + std::fmt::Debug> OperatorsPort for OperatorsAdapter<U> {}
