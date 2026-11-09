//! The `list_operators` use case: it asks the operator names for each operator's description.

use super::errors::ListOperatorsError;
use super::interface::ListOperatorsUseCase;
use super::types::{ListOperatorsReport, OperatorSummary};
use crate::application::metadata::service::kernel_metadata;
use ryo_execution::inbound::use_cases::metadata::interface::MetadataUseCase;

/// `list_operators`; the operators are the kernel's own, asked through the `metadata` application service, so it calls no port.
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
        let operators = kernel_metadata()
            .operators()
            .into_iter()
            .map(|operator| kernel_metadata().describe(*operator))
            .map(|description| OperatorSummary {
                name: description.name,
                summary: description.summary,
            })
            .collect();
        Ok(ListOperatorsReport { operators })
    }
}
