//! Only borrowed semantic inputs cross this boundary.
use ryo_operators::domain::contract::types::{OperatorAnswer, OperatorCall, PortableOperatorFault};

/// The engine keeps the scratch row, distinct set and output sink.
pub trait OperatorsPort: Send + Sync + std::fmt::Debug {
    /// Apply one portable semantic request without deciding publication.
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault>;
}
