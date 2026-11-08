//! What the engine asks of the operators capability: one borrowed call in, one pure meaning out; execution authority never crosses it.
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::types::{OperatorAnswer, OperatorCall};

/// What every `operators` adapter implements.
pub trait OperatorsPort: Send + Sync + std::fmt::Debug {
    /// Runs the operator the call names over its borrowed rows and answers its meaning.
    ///
    /// # Errors
    ///
    /// The portable fault the operator refuses with.
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault>;
}
