//! Request preparation composes the execution owner and injected semantic port.
use super::{
    interface::ExecutionUseCase,
    types::{PreparedProgram, PreparedRequest},
};
use crate::{
    application::file_dag::program::facade, domain::operating::errors::OperatingError,
    outbound::derived::interface::DerivedPort,
};
use std::sync::Arc;
pub struct ExecutionUseCaseImpl {
    derived: Arc<dyn DerivedPort>,
}
impl ExecutionUseCaseImpl {
    pub fn new(derived: Arc<dyn DerivedPort>) -> Self {
        Self { derived }
    }
}
impl ExecutionUseCase for ExecutionUseCaseImpl {
    fn prepare(&self, request: PreparedRequest) -> Result<PreparedProgram, OperatingError> {
        let inner = facade::prepare(
            request.compiled,
            &request.rules,
            &request.digests,
            &request.hosted,
            Arc::clone(&self.derived),
            request.observations,
        )?;
        Ok(PreparedProgram { inner })
    }
}
