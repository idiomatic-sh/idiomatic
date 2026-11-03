//! The cohesive operator capability interface.
use crate::domain::contract::types::{OperatorAnswer, OperatorCall, PortableOperatorFault};

/// Pure row semantics without scheduling, retained storage or publication authority.
pub trait OperatorsUseCase: Sync {
    /// Evaluates one borrowed semantic request into caller-owned scratch.
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault>;
}
