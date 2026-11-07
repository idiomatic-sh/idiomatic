//! The fake of the `receipts` port: what the core's tests stand in for it with. It records through the local
//! filesystem adapter, so a row it appends is the row the real adapter appends.
use super::super::interface::ReceiptsPort;
use super::super::errors::ReceiptsError;
use super::super::types::{CapturedProcess, CaseReceipts, ReceiptPath};
use ryo_benchmark_contracts::domain::documents::types::observation_document::value::ObservationDocument;

/// What the core's tests stand in for `receipts` with.
#[derive(Clone, Debug)]
pub struct ReceiptsFake {
    /// The destination this fake reports.
    pub case_receipts: CaseReceipts,
}

impl Default for ReceiptsFake {
    fn default() -> Self {
        Self {
            case_receipts: CaseReceipts::Unrecorded,
        }
    }
}

impl ReceiptsPort for ReceiptsFake {
    fn case_receipts(&self) -> CaseReceipts {
        self.case_receipts.clone()
    }

    fn record_case(
        &self,
        receipt: ReceiptPath<'_>,
        record: ObservationDocument,
        process: Option<CapturedProcess<'_>>,
    ) -> Result<(), ReceiptsError> {
        crate::outbound::receipts::factory::local(self.case_receipts.clone())
            .record_case(receipt, record, process)
    }

    fn case_observations(
        &self,
        receipt: ReceiptPath<'_>,
    ) -> Result<Vec<ObservationDocument>, ReceiptsError> {
        crate::outbound::receipts::factory::local(self.case_receipts.clone()).case_observations(receipt)
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
