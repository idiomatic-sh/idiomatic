//! The composition owns the destination stream.
use super::{adapters::bmf::adapter::EvidenceBmfAdapter, interface::EvidencePort};
use std::io::Write;

/// Bind BMF publication to the supplied stream.
#[must_use]
pub fn bmf(output: Box<dyn Write + Send>) -> impl EvidencePort {
    EvidenceBmfAdapter::new(output)
}
