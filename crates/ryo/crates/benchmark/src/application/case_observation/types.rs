//! Data owned by the writing of one observation row.
use ryo_benchmark_contracts::domain::documents::types::invocation_document::value::InvocationDocument;
use ryo_benchmark_contracts::domain::measurements::types::ProcessOutput;

/// The raw process a row keeps beside it.
pub struct ObservedProcess<'a> {
    /// The raw invocation record.
    pub invocation: &'a InvocationDocument,
    /// The exact process output.
    pub output: &'a ProcessOutput,
}

/// Whether a row keeps a raw process.
pub enum ProcessEvidence<'a> {
    /// The row keeps no raw process.
    Absent,
    /// The row keeps this raw process.
    Kept(ObservedProcess<'a>),
}
