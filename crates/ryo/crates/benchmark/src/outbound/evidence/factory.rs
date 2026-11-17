//! The composition owns the destination stream.
use super::{adapters::bmf::adapter::EvidenceBmfAdapter, interface::EvidencePort};
use std::io::Write;

/// Bind BMF publication to the supplied stream, and the comparison evidence to its own stream.
#[must_use]
pub fn bmf(output: Box<dyn Write + Send>, comparison: Box<dyn Write + Send>) -> impl EvidencePort {
    EvidenceBmfAdapter::new(super::adapters::bmf::connection::EvidenceBmfConnection {
        output: super::adapters::bmf::connection::DocumentDestination::Stream(
            std::sync::Mutex::new(output),
        ),
        comparison: std::sync::Mutex::new(comparison),
        document: std::sync::Mutex::new(Some(
            super::adapters::bmf::types::MeasureDocument::default(),
        )),
    })
}

/// Bind BMF publication to the file at `destination`, appended to when the run finishes; the comparison evidence goes
/// nowhere.
#[must_use]
pub fn bmf_appending(destination: std::path::PathBuf) -> impl EvidencePort {
    EvidenceBmfAdapter::new(super::adapters::bmf::connection::EvidenceBmfConnection {
        output: super::adapters::bmf::connection::DocumentDestination::Appended(destination),
        comparison: std::sync::Mutex::new(Box::new(std::io::sink())),
        document: std::sync::Mutex::new(Some(
            super::adapters::bmf::types::MeasureDocument::default(),
        )),
    })
}

/// Bind the bench's one BMF document to a new file at `document`, created when the run finishes, and the comparison
/// evidence to its own stream.
#[must_use]
pub fn bmf_document(
    document: std::path::PathBuf,
    comparison: Box<dyn Write + Send>,
) -> impl EvidencePort {
    EvidenceBmfAdapter::new(super::adapters::bmf::connection::EvidenceBmfConnection {
        output: super::adapters::bmf::connection::DocumentDestination::Created(document),
        comparison: std::sync::Mutex::new(comparison),
        document: std::sync::Mutex::new(Some(
            super::adapters::bmf::types::MeasureDocument::default(),
        )),
    })
}
