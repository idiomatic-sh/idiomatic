//! The `operators` port, implemented by the `operators` crate's use case: the call and the answer are the values the core names,
//! so the adapter passes them through.
use crate::outbound::operators::interface::OperatorsPort;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;
use ryo_operators::inbound::use_cases::operators::types::{OperatorAnswer, OperatorCall};

/// The `use_case` `operators` adapter.
#[derive(Debug)]
pub(crate) struct OperatorsUseCaseAdapter<U> {
    /// The `operators` use case of the operators crate, which the edge built.
    operators: U,
}

impl<U> OperatorsUseCaseAdapter<U> {
    /// The adapter over the `operators` use case the edge built.
    pub(crate) const fn new(operators: U) -> Self {
        Self { operators }
    }
}

impl<U: OperatorsUseCase + Send + Sync + std::fmt::Debug> OperatorsPort for OperatorsUseCaseAdapter<U> {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        self.operators.operators(call)
    }
}
