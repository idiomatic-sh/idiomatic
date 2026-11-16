//! The composition owns the destination stream.
use super::{adapters::bmf::adapter::EvidenceBmfAdapter, interface::EvidencePort};
use std::io::Write;

/// Bind BMF publication to the supplied stream, and the comparison evidence to its own stream.
#[must_use]
pub fn bmf(output: Box<dyn Write + Send>, comparison: Box<dyn Write + Send>) -> impl EvidencePort {
    EvidenceBmfAdapter::new(super::adapters::bmf::connection::EvidenceBmfConnection {
        output: std::sync::Mutex::new(output),
        comparison: std::sync::Mutex::new(comparison),
        document: std::sync::Mutex::new(Some(
            super::adapters::bmf::types::MeasureDocument::default(),
        )),
    })
}

/// Bind BMF publication to the file at `destination`, appended to, or to nowhere when the run names none; the comparison
/// evidence goes nowhere.
///
/// # Errors
/// Refuses a destination that cannot be opened for appending.
pub fn bmf_appending(
    destination: Option<&std::path::Path>,
) -> Result<impl EvidencePort + use<>, ryo_benchmark_contracts::domain::reports::errors::ReportsError>
{
    Ok(EvidenceBmfAdapter::new(
        super::adapters::bmf::connection::EvidenceBmfConnection::appending(destination)?,
    ))
}

/// Bind the bench's one BMF document to a new file at `document`, and the comparison evidence to its own stream.
///
/// # Errors
/// Refuses a document file that cannot be created.
pub fn bmf_document(
    document: &std::path::Path,
    comparison: Box<dyn Write + Send>,
) -> Result<impl EvidencePort + use<>, ryo_benchmark_contracts::domain::reports::errors::ReportsError>
{
    Ok(EvidenceBmfAdapter::new(
        super::adapters::bmf::connection::EvidenceBmfConnection::document(document, comparison)?,
    ))
}
