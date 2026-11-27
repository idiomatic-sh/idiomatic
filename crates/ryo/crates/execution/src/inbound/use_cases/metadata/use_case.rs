//! The metadata use case, over the one list of operators and their descriptions.
use super::interface::MetadataUseCase;
use ryo_values::domain::descriptions::types::OperatorDescription;
use ryo_values::domain::query_operators::types::QueryOperator;

/// Stateless access to the operator catalog.
#[derive(Clone, Copy, Debug, Default)]
pub struct MetadataUseCaseImpl;

impl MetadataUseCase for MetadataUseCaseImpl {
    fn operators(&self) -> &'static [QueryOperator] {
        &ryo_values::domain::query_operators::constants::OPERATORS
    }
    fn describe(&self, operator: QueryOperator) -> OperatorDescription {
        ryo_execution_domain::domain::operator::constants::describe(operator)
    }
}
