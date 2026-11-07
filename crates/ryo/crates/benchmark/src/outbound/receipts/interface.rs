//! What the `receipts` port asks of the outside, in the core's own vocabulary: an observation row and the raw process bytes
//! kept beside it. Where the receipts go is the adapter's setting, never asked here.
use super::errors::ReceiptsError;
use super::types::{CapturedProcess, CaseReceipts, ReceiptPath};
use ryo_benchmark_contracts::domain::documents::types::observation_document::value::ObservationDocument;

/// What every `receipts` adapter implements.
pub trait ReceiptsPort {
    /// Where this run appends its case receipts, as the edge chose.
    fn case_receipts(&self) -> CaseReceipts;

    /// Append an application-owned observation and preserve optional raw process bytes.
    ///
    /// # Errors
    /// Refuses unsafe receipt names or failed artifact persistence.
    fn record_case(
        &self,
        receipt: ReceiptPath<'_>,
        record: ObservationDocument,
        process: Option<CapturedProcess<'_>>,
    ) -> Result<(), ReceiptsError>;
}
