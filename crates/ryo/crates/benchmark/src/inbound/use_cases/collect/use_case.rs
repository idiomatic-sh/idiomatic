//! The isolated process composes protocol validation with direct child execution.
use super::interface::CollectUseCase;
use crate::{
    domain::diagnostics::errors::DiagnosticsError,
    outbound::{
        collection::{interface::CollectionPort, types::CollectorMessage},
        execution::interface::ExecutionPort,
    },
};
use std::sync::Arc;
/// Collector operation with explicit boundaries.
pub struct CollectUseCaseImpl {
    codec: Arc<dyn CollectionPort>,
    execution: Arc<dyn ExecutionPort>,
}
impl CollectUseCaseImpl {
    /// Compose a protocol and direct child observer.
    #[must_use]
    pub fn new(codec: Arc<dyn CollectionPort>, execution: Arc<dyn ExecutionPort>) -> Self {
        Self { codec, execution }
    }
}
impl std::fmt::Debug for CollectUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
impl CollectUseCase for CollectUseCaseImpl {
    fn collect(&self, request: &CollectorMessage) -> Result<CollectorMessage, DiagnosticsError> {
        let invocation = self.codec.invocation(request)?;
        self.codec.response(self.execution.run(invocation))
    }
}
