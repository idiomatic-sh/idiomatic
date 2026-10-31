//! An injected destination stream owned by the evidence adapter.
use std::{io::Write, sync::Mutex};

/// Resource lifetime is bounded by the configured adapter instance.
pub(crate) struct EvidenceBmfConnection {
    pub(crate) output: Mutex<Box<dyn Write + Send>>,
}
impl std::fmt::Debug for EvidenceBmfConnection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
