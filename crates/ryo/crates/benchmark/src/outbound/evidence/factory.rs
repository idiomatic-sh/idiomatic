//! The composition owns the destination stream.
use super::{adapters::bmf::adapter::EvidenceBmfAdapter, interface::EvidencePort};
use std::io::Write;

/// Bind BMF publication to the supplied stream, and the comparison evidence to its own stream.
#[must_use]
pub fn bmf(output: Box<dyn Write + Send>, comparison: Box<dyn Write + Send>) -> impl EvidencePort {
    EvidenceBmfAdapter::new(super::adapters::bmf::connection::EvidenceBmfConnection {
        output: std::sync::Mutex::new(output),
        comparison: std::sync::Mutex::new(comparison),
        parity: std::sync::Mutex::default(),
        document: std::sync::Mutex::new(Some(super::adapters::bmf::types::Document::default())),
    })
}

/// Bind BMF publication to the file at `destination`, appended to, or to nowhere when the run names none; the comparison
/// evidence goes nowhere.
///
/// # Errors
/// Refuses a destination that cannot be opened for appending.
pub fn bmf_to(destination: Option<&std::path::Path>) -> Result<impl EvidencePort, std::io::Error> {
    let output: Box<dyn Write + Send> = match destination {
        Some(path) => Box::new(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)?,
        ),
        None => Box::new(std::io::sink()),
    };
    Ok(bmf(output, Box::new(std::io::sink())))
}
