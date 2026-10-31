//! Audit delegates external artifact validation to its suite boundary.
use super::interface::AuditUseCase;
use crate::domain::suite::types::Audit;
use crate::domain::{diagnostics::errors::Error, suite::types::Tool};
use crate::outbound::suite::interface::SuitePort;
use std::{path::Path, sync::Arc};
pub struct AuditUseCaseImpl {
    suite: Arc<dyn SuitePort>,
}
impl AuditUseCaseImpl {
    pub fn new(suite: Arc<dyn SuitePort>) -> Self {
        Self { suite }
    }
}
impl AuditUseCase for AuditUseCaseImpl {
    fn audit(&self, directory: &Path, tool: &Tool) -> Result<Audit, Error> {
        self.suite.audit(directory, tool)
    }
}
