//! The fake of the `observer` port: what the core's tests stand in for it with.

use crate::outbound::observer::interface::ObserverPort;
use crate::outbound::observer::types::{FedTotals, FileHandovers};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::types::{LogicalCounts, RowsFed};
use ryo_values::domain::runs::types::FileHandover;
use std::sync::Mutex;

/// What the core's tests stand in for `observer` with: every report in the order it came, and their sum.
#[derive(Debug, Default)]
pub struct ObserverFake {
    /// Each report: the rule and the rows each of its steps was fed.
    pub reports: Mutex<Vec<(RuleIndex, Vec<RowsFed>)>>,
    /// Each handover, in the order it came.
    pub handed: Mutex<Vec<FileHandover>>,
}

impl ObserverPort for ObserverFake {
    fn fed(&self, rule: RuleIndex, counts: LogicalCounts) -> Result<(), OperatingError> {
        self.reports
            .lock()
            .map_err(|_| OperatingError::NotRun)?
            .push((rule, counts.fed.into_vec()));
        Ok(())
    }
    fn totals(&self) -> Result<FedTotals, OperatingError> {
        let reports = self.reports.lock().map_err(|_| OperatingError::NotRun)?;
        let mut totals = FedTotals::default();
        for (rule, fed) in reports.iter() {
            if totals.0.len() <= rule.0 {
                totals.0.resize_with(rule.0 + 1, Vec::new);
            }
            let steps = totals.0.get_mut(rule.0).ok_or(OperatingError::NotRun)?;
            if steps.len() < fed.len() {
                steps.resize(fed.len(), RowsFed::default());
            }
            for (total, more) in steps.iter_mut().zip(fed) {
                for (slot, each) in total.0.iter_mut().zip(more.0) {
                    *slot = slot.saturating_add(each);
                }
            }
        }
        Ok(totals)
    }
    fn handed(&self, handover: FileHandover) -> Result<(), OperatingError> {
        self.handed
            .lock()
            .map_err(|_| OperatingError::NotRun)?
            .push(handover);
        Ok(())
    }
    fn handovers(&self) -> Result<FileHandovers, OperatingError> {
        let mut told = self
            .handed
            .lock()
            .map_err(|_| OperatingError::NotRun)?
            .clone();
        told.sort_unstable();
        Ok(FileHandovers(told))
    }
}

