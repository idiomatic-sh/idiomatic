//! Append receipt rows and raw process bytes to the local filesystem.
use super::writer;
use crate::outbound::receipts::errors::ReceiptsError;
use crate::outbound::receipts::interface::ReceiptsPort;
use crate::outbound::receipts::types::{CapturedProcess, CaseReceipts, ReceiptPath};
use ryo_benchmark_contracts::domain::documents::types::observation_document::value::ObservationDocument;

/// Appends receipts to the destination the edge chose.
#[derive(Debug)]
pub(crate) struct ReceiptsLocalAdapter {
    case_receipts: CaseReceipts,
}
impl ReceiptsLocalAdapter {
    /// Bind the receipt destination chosen explicitly by executable composition.
    pub(crate) fn new(case_receipts: CaseReceipts) -> Self {
        Self { case_receipts }
    }
}
impl ReceiptsPort for ReceiptsLocalAdapter {
    fn case_receipts(&self) -> CaseReceipts {
        self.case_receipts.clone()
    }

    fn record_case(
        &self,
        ReceiptPath(receipt): ReceiptPath<'_>,
        record: ObservationDocument,
        process: Option<CapturedProcess<'_>>,
    ) -> Result<(), ReceiptsError> {
        writer::record(receipt, record, process)
    }

    fn case_observations(
        &self,
        ReceiptPath(receipt): ReceiptPath<'_>,
    ) -> Result<Vec<ObservationDocument>, ReceiptsError> {
        writer::read(receipt)
    }
}
