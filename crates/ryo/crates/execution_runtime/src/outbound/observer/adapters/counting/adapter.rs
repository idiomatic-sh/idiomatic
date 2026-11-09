//! The observer port, implemented by counting the rows each step is fed.

use crate::outbound::observer::interface::ObserverPort;
use crate::outbound::observer::types::FedTotals;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operator::types::{LogicalCounts, RowsFed};
use super::constants::COUNTED_FED;
use super::types::CountedFed;

/// The counting observer: what it is told is added up by rule and step, for at most `COUNTED_FED` rules. Addition commutes, so the totals do not depend on the
/// order threads report in.
#[derive(Debug, Default)]
pub(crate) struct ObserverCountingAdapter {
    totals: CountedFed,
}

impl ObserverPort for ObserverCountingAdapter {
    fn fed(&self, rule: RuleIndex, counts: LogicalCounts) {
        if rule.0 >= COUNTED_FED {
            return;
        }
        let Ok(mut totals) = self.totals.0.lock() else {
            return;
        };
        let rules = rule.0 + 1;
        if totals.0.len() < rules {
            totals.0.resize_with(rules, Vec::new);
        }
        let Some(steps) = totals.0.get_mut(rule.0) else {
            return;
        };
        if steps.len() < counts.fed.len() {
            steps.resize(counts.fed.len(), RowsFed::default());
        }
        for (total, more) in steps.iter_mut().zip(counts.fed.iter()) {
            for (slot, each) in total.0.iter_mut().zip(more.0) {
                *slot = slot.saturating_add(each);
            }
        }
    }

    fn totals(&self) -> FedTotals {
        self.totals
            .0
            .lock()
            .map(|totals| totals.clone())
            .unwrap_or_default()
    }
}
