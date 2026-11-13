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
    })
}

