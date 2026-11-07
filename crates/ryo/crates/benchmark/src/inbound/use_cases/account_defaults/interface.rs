//! The accounting of a tool's shipped defaults against its audit.
use crate::domain::suite::types::audit::value::Audit;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use ryo_benchmark_contracts::domain::workloads::types::Tool;
/// Refuse a tool whose shipped defaults are not all accounted for.
pub trait AccountDefaultsUseCase {
    /// Every default must be authored, or deferred or pending with an assessed reason.
    ///
    /// # Errors
    /// Refuses an audit without authored rules and a default that is none of the three.
    fn account_defaults(&self, audit: &Audit, tool: &Tool) -> Result<(), ReportsError>;
}
