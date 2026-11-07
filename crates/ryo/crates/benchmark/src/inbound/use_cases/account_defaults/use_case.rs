//! The accounting holds no boundary: its facts are the audit and the tool it is given.
use super::interface::AccountDefaultsUseCase;
use crate::domain::suite::types::audit::value::Audit;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::Tool;
/// Accounts for the defaults of one tool.
#[derive(Clone, Copy, Debug, Default)]
pub struct AccountDefaultsUseCaseImpl;
impl AccountDefaultsUseCaseImpl {
    /// Construct the accounting; it has no boundary to bind.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}
impl AccountDefaultsUseCase for AccountDefaultsUseCaseImpl {
    fn account_defaults(&self, audit: &Audit, tool: &Tool) -> Result<(), Error> {
        crate::domain::suite::service::require_accounted(audit, tool)
    }
}
