//! How many rules a tool ships, defers and leaves pending, beside how many are authored.
use ryo_benchmark_contracts::domain::workloads::types::Tool;
use ryo_benchmark_contracts::domain::workloads::types::rule_count::value::RuleCount;

/// How many rules the tool ships, defers and leaves pending, and how many of the shipped ones are authored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuleTally {
    /// The shipped defaults.
    pub defaults: RuleCount,
    /// The deferred defaults.
    pub deferred: RuleCount,
    /// The pending defaults.
    pub pending: RuleCount,
    /// The authored rules.
    pub authored: RuleCount,
}

impl RuleTally {
    /// Count the tool's defaults, deferrals and pending rules beside the authored total.
    #[must_use]
    pub fn of(tool: &Tool, authored: RuleCount) -> Self {
        Self {
            defaults: RuleCount(tool.defaults.len()),
            deferred: RuleCount(tool.deferred.len()),
            pending: RuleCount(tool.pending.len()),
            authored,
        }
    }
}
