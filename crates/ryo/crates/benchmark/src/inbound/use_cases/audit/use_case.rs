//! Audit delegates external artifact validation to its suite boundary.
use super::interface::AuditUseCase;
use crate::domain::suite::types::audit::value::Audit;
use crate::outbound::suite::interface::SuitePort;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::Tool;
use std::{path::Path, sync::Arc};
/// Audit benchmark artifacts against a pinned default profile.
pub struct AuditUseCaseImpl {
    /// Artifact validation and exact content fingerprinting boundary.
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
        crate::application::run::service::audit(self.suite.as_ref(), directory, tool)
    }
}

impl std::fmt::Debug for AuditUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
