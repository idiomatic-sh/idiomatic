//! An injected destination stream owned by the evidence adapter.
use super::types::{ParityCounts, ReportName};
use std::{collections::BTreeMap, io::Write, sync::Mutex};

/// Resource lifetime is bounded by the configured adapter instance.
pub(crate) struct EvidenceBmfConnection {
    /// Serialized access to the externally supplied evidence destination.
    pub output: Mutex<Box<dyn Write + Send>>,
    /// Serialized access to the externally supplied comparison evidence destination.
    pub comparison: Mutex<Box<dyn Write + Send>>,
    /// The counts of every parity report published, by reference name: one document holds them all, written when the
    /// adapter is done.
    pub parity: Mutex<BTreeMap<ReportName, ParityCounts>>,
}
impl std::fmt::Debug for EvidenceBmfConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
