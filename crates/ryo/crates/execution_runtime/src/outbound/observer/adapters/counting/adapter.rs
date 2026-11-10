//! The observer port, implemented by counting the rows each step is fed.

use super::constants::COUNTED_FED;
use super::types::CountedFed;
use crate::outbound::observer::interface::ObserverPort;
use crate::outbound::observer::types::FedTotals;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::types::{LogicalCounts, RowsFed};

/// The counting observer: what it is told is added up by rule and step, for at most `COUNTED_FED` rules. Addition commutes, so
/// the totals do not depend on the order threads report in.
#[derive(Debug, Default)]
pub(crate) struct ObserverCountingAdapter {
    totals: CountedFed,
}

impl ObserverPort for ObserverCountingAdapter {
    fn fed(&self, rule: RuleIndex, counts: LogicalCounts) -> Result<(), OperatingError> {
        if rule.0 >= COUNTED_FED {
            return Err(OperatingError::ShapeMismatch);
        }
        let mut totals = self.totals.0.lock().map_err(|_| OperatingError::NotRun)?;
        if totals.0.len() <= rule.0 {
            totals.0.resize_with(rule.0 + 1, Vec::new);
        }
        let steps = totals.0.get_mut(rule.0).ok_or(OperatingError::NotRun)?;
        if steps.len() < counts.fed.len() {
            steps.resize(counts.fed.len(), RowsFed::default());
        }
        for (total, more) in steps.iter_mut().zip(counts.fed.iter()) {
            for (slot, each) in total.0.iter_mut().zip(more.0) {
                *slot = slot
                    .checked_add(each)
                    .ok_or(OperatingError::ShapeMismatch)?;
            }
        }
        Ok(())
    }

    fn totals(&self) -> Result<FedTotals, OperatingError> {
        self.totals
            .0
            .lock()
            .map(|totals| totals.clone())
            .map_err(|_| OperatingError::NotRun)
    }
}
