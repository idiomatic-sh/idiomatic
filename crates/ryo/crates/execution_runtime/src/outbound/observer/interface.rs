//! What the `observer` port asks of the outside, in the core's own vocabulary.

use super::types::FedTotals;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::types::LogicalCounts;

/// What the engine reports about its own work, to a caller that asked for it. Nothing the engine answers depends on it.
pub trait ObserverPort: Send + Sync {
    /// The rows each step of `rule` was fed, at the point the caller's counts are settled.
    ///
    /// # Errors
    ///
    /// The observer cannot keep the report: the run that asked for it stops rather than answer a count short.
    fn fed(&self, rule: RuleIndex, counts: LogicalCounts) -> Result<(), OperatingError>;
    /// What the observer has been told so far, added up by rule then step.
    ///
    /// # Errors
    ///
    /// The observer lost what it was told.
    fn totals(&self) -> Result<FedTotals, OperatingError>;
}
