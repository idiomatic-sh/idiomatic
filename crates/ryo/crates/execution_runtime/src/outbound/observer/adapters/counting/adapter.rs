//! The observer port, implemented by counting the rows each step is fed.

use crate::outbound::observer::interface::ExecutionObserver;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operator::types::{LogicalCounts, RowsFed};
use std::sync::atomic::{AtomicU64, Ordering};

/// Totals of the rows each step of each rule was fed. Addition commutes, so the totals do not depend on the order threads report in.
#[derive(Debug)]
pub struct CountingObserver {
    totals: Box<[Box<[[AtomicU64; 3]]>]>,
}

impl CountingObserver {
    /// Nothing counted yet, with room for the steps of each rule.
    pub(crate) fn new(steps: &[usize]) -> Self {
        Self {
            totals: steps
                .iter()
                .map(|steps| (0..*steps).map(|_| Default::default()).collect())
                .collect(),
        }
    }

    /// The rows each step of each rule was fed, by rule.
    #[must_use]
    pub fn totals(&self) -> Vec<Vec<RowsFed>> {
        self.totals
            .iter()
            .map(|steps| {
                steps
                    .iter()
                    .map(|step| RowsFed(step.each_ref().map(|input| input.load(Ordering::Relaxed))))
                    .collect()
            })
            .collect()
    }
}

impl ExecutionObserver for CountingObserver {
    fn fed(&self, rule: RuleIndex, counts: LogicalCounts) {
        let Some(steps) = self.totals.get(rule.0) else {
            return;
        };
        for (step, fed) in steps.iter().zip(counts.fed.iter()) {
            for (total, more) in step.iter().zip(fed.0) {
                total.fetch_add(more, Ordering::Relaxed);
            }
        }
    }
}
