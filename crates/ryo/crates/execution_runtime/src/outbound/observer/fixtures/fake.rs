//! The fake of the `observer` port: what the core's tests stand in for it with.

use crate::outbound::observer::interface::ExecutionObserver;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operator::types::LogicalCounts;
use std::sync::Mutex;

/// What the core's tests stand in for `observer` with: every report, in the order it came.
#[derive(Debug, Default)]
pub struct ObserverFake {
    /// Each report: the rule and the rows each of its steps was fed.
    pub reports: Mutex<Vec<(RuleIndex, Vec<[u64; 3]>)>>,
}

impl ExecutionObserver for ObserverFake {
    fn fed(&self, rule: RuleIndex, counts: LogicalCounts) {
        if let Ok(mut reports) = self.reports.lock() {
            reports.push((rule, counts.fed.iter().map(|fed| fed.0).collect()));
        }
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod fake_test;
