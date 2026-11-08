//! Private revocation state; consumers can only observe liveness.
/// Read-only revocation witness, and the generation itself: the owner issues one lease per resident population, so
/// the lease's allocation is that population's generation. Only the population owner can invalidate it; semantic
/// callbacks never receive a mutable atomic or a revival operation.
#[derive(Debug)]
pub(in crate::application::file_dag) struct GenerationLease {
    live: std::sync::atomic::AtomicBool,
}
impl GenerationLease {
    pub(in crate::application::file_dag::relations) fn new() -> Self {
        Self {
            live: std::sync::atomic::AtomicBool::new(true),
        }
    }
    pub(in crate::application::file_dag::relations) fn invalidate(&self) {
        self.live.store(false, std::sync::atomic::Ordering::Release);
    }
    /// Whether two leases are one generation. The owner issues a fresh lease for each population and every proof
    /// keeps the lease it was issued under, so no released allocation can be reused for a later generation.
    pub(in crate::application::file_dag) fn same(
        this: &std::sync::Arc<Self>,
        other: &std::sync::Arc<Self>,
    ) -> crate::application::file_dag::relations::types::Sameness {
        if std::sync::Arc::ptr_eq(this, other) {
            crate::application::file_dag::relations::types::Sameness::Same
        } else {
            crate::application::file_dag::relations::types::Sameness::Different
        }
    }
    pub(in crate::application::file_dag) fn is_live(&self) -> crate::application::file_dag::relations::types::Liveness {
        if self.live.load(std::sync::atomic::Ordering::Acquire) {
            crate::application::file_dag::relations::types::Liveness::Live
        } else {
            crate::application::file_dag::relations::types::Liveness::Released
        }
    }
}
