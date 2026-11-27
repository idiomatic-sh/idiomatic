//! What the metadata use case promises: the operators the language has, and how each describes itself.
use ryo_values::domain::descriptions::types::OperatorDescription;
use ryo_values::domain::query_operators::types::QueryOperator;

// The vocabulary the core's lint types take from here.
pub use ryo_execution_domain::domain::coverage::types::Message;
pub use ryo_execution_domain::domain::coverage::types::Unknown;
pub use ryo_execution_domain::domain::features::types::FeatureRows;

/// What the operator catalog answers.
pub trait MetadataUseCase {
    /// Every operator, in the order the documentation lists them.
    fn operators(&self) -> &'static [QueryOperator];
    /// What an operator is, what it takes and an example.
    fn describe(&self, operator: QueryOperator) -> OperatorDescription;
}
