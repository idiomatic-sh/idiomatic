//! What the `inventory` port asks of the outside, in the core's own vocabulary: plain data in and out,
//! and the handles (traits this file declares) it hands back. It never takes an adapter's
//! configuration (a directory, a budget, a client): the port's factory takes that.
use super::types::ContextFiles;
use super::types::RawArtifacts;
use super::types::ReceiptFile;
use super::types::Registration;
use super::types::Row;
use super::types::Summary;
use ryo_benchmark_contracts::domain::documents::types::invocation_document::value::InvocationDocument;
use ryo_benchmark_contracts::domain::documents::types::observation_document::value::ObservationDocument;
use ryo_benchmark_contracts::domain::measurements::types::CapturedBytes;
use ryo_benchmark_contracts::domain::receipts::types::ReceiptAudit;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::path::Path;

/// What every `inventory` adapter implements: registrations in, exact observations and results out.
pub trait InventoryPort {
    /// Decode the generated invocation registration.
    ///
    /// # Errors
    /// Returns invalid artifact, encoding or filesystem errors.
    fn load(&self, path: &Path) -> Result<Registration, Error>;
    /// Begin a fresh report while retaining already staged reproductions.
    ///
    /// # Errors
    /// Refuses an output that already holds a summary, and filesystem failures.
    fn begin(&self, output: &Path) -> Result<(), Error>;
    /// Atomically replace the complete summary.
    ///
    /// # Errors
    /// Returns encoding or filesystem errors.
    fn publish(&self, output: &Path, summary: &Summary) -> Result<(), Error>;
    /// Refuse receipt reuse and prepare one rule's observation directory.
    ///
    /// # Errors
    /// Refuses a directory that already holds receipts, and filesystem failures.
    fn prepare(&self, observations: &Path) -> Result<(), Error>;
    /// Keep the exact child streams and say whether a native case receipt exists beside them.
    ///
    /// # Errors
    /// Returns filesystem errors.
    fn capture(
        &self,
        observations: &Path,
        stdout: &CapturedBytes,
        stderr: &CapturedBytes,
    ) -> Result<ReceiptFile, Error>;
    /// Keep a terminal row beside its exact native observations.
    ///
    /// # Errors
    /// Returns encoding or filesystem errors.
    fn result(&self, observations: &Path, row: &Row) -> Result<(), Error>;
    /// Decode one raw invocation record kept beside a captured process.
    ///
    /// # Errors
    /// Refuses malformed or unknown record fields.
    fn decode_invocation(&self, bytes: &CapturedBytes) -> Result<InvocationDocument, Error>;
    /// Read only the raw process artifacts the rows of one receipt file reference.
    ///
    /// # Errors
    /// Refuses unsafe paths, stale or missing bytes and unreferenced extras.
    fn raw_artifacts(
        &self,
        receipt: &Path,
        records: &[ObservationDocument],
    ) -> Result<RawArtifacts, Error>;
    /// List every regular file of a registered context, refusing any other entry.
    ///
    /// # Errors
    /// Refuses a context that is not canonical, or holds a non-regular entry.
    fn context_files(&self, context: &Path) -> Result<ContextFiles, Error>;
    /// Encode the answer of an offline receipt audit as one line for the caller's standard output.
    ///
    /// # Errors
    /// Refuses an audit the document encoding cannot represent.
    fn audit_report(&self, audit: &ReceiptAudit) -> Result<CapturedBytes, Error>;
}
