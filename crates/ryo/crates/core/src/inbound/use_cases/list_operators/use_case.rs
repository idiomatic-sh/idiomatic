//! The `list_operators` use case: it asks the operator names for each operator's description.

use super::errors::ListOperatorsError;
use super::interface::ListOperatorsUseCase;
use super::types::{ListOperatorsReport, OperatorSummary};
use ryo_execution::inbound::use_cases::metadata::interface::MetadataUseCase;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;

/// `list_operators`; the operators are the domain's own, so it calls no port.
#[derive(Clone, Copy, Debug, Default)]
pub struct ListOperatorsUseCaseImpl;

impl ListOperatorsUseCaseImpl {
    /// `list_operators`, which calls no port.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl ListOperatorsUseCase for ListOperatorsUseCaseImpl {
    fn list_operators(&self) -> Result<ListOperatorsReport, ListOperatorsError> {
        let operators = MetadataUseCaseImpl
            .operators()
            .into_iter()
            .map(|operator| MetadataUseCaseImpl.describe(*operator))
            .map(|description| OperatorSummary {
                name: description.name,
                summary: description.summary,
            })
            .collect();
        Ok(ListOperatorsReport { operators })
    }
}
