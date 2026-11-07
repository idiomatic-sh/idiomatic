//! The `list_operators` use case: it asks the operator names for each operator's description.

use super::errors::ListOperatorsError;
use super::interface::ListOperatorsUseCase;
use super::types::{ListOperatorsReport, OperatorSummary};
use ryo_execution::inbound::use_cases::metadata::interface::MetadataUseCase;
use std::sync::Arc;

/// `list_operators`, over the metadata use case that knows the operators.
pub struct ListOperatorsUseCaseImpl {
    /// Knows the operators and describes each.
    metadata: Arc<dyn MetadataUseCase + Send + Sync>,
}

impl std::fmt::Debug for ListOperatorsUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl ListOperatorsUseCaseImpl {
    /// `list_operators` over the metadata use case the edge injects.
    #[must_use]
    pub fn new(metadata: Arc<dyn MetadataUseCase + Send + Sync>) -> Self {
        Self { metadata }
    }
}

impl ListOperatorsUseCase for ListOperatorsUseCaseImpl {
    fn list_operators(&self) -> Result<ListOperatorsReport, ListOperatorsError> {
        let operators = self
            .metadata
            .operators()
            .into_iter()
            .map(|operator| self.metadata.describe(*operator))
            .map(|description| OperatorSummary {
                name: description.name,
                summary: description.summary,
            })
            .collect();
        Ok(ListOperatorsReport { operators })
    }
}
