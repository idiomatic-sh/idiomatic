//! Audit delegates external artifact validation to its suite boundary.
use super::interface::AuditUseCase;
use crate::domain::suite::types::audit::value::Audit;
use crate::domain::{diagnostics::errors::DiagnosticsError as Error, suite::types::Tool};
use crate::outbound::suite::interface::SuitePort;
use std::{path::Path, sync::Arc};
/// Audit benchmark artifacts against a pinned default profile.
pub struct AuditUseCaseImpl {
    suite: Arc<dyn SuitePort>,
}
impl AuditUseCaseImpl {
    /// Construct this operation with the dependencies chosen by its composition.
    #[must_use]
    pub fn new(suite: Arc<dyn SuitePort>) -> Self {
        Self { suite }
    }
}
impl AuditUseCase for AuditUseCaseImpl {
    fn audit(&self, directory: &Path, tool: &Tool) -> Result<Audit, Error> {
        self.suite
            .audit(crate::outbound::suite::types::ArtifactPath(directory), tool)
    }
}

impl std::fmt::Debug for AuditUseCaseImpl {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
