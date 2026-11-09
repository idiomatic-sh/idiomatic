//! What the `observer` port asks of the outside, in the core's own vocabulary.

use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operator::types::LogicalCounts;

/// What the engine reports about its own work, to a caller that asked for it. Nothing the engine answers depends on it.
pub trait ExecutionObserver: Send + Sync {
    /// The rows each step of `rule` was fed, at the point the caller's counts are settled.
    fn fed(&self, rule: RuleIndex, counts: LogicalCounts);
}
