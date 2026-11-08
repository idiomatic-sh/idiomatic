//! Private revocation state; consumers can only observe liveness.
/// Read-only revocation witness. Only the population owner can invalidate it;
/// semantic callbacks never receive a mutable atomic or a revival operation.
pub struct GenerationLease {
    live: std::sync::atomic::AtomicBool,
}
impl GenerationLease {
    pub(in crate::application::file_dag::relations) fn new() -> Self {
        Self {
            live: std::sync::atomic::AtomicBool::new(true),
        }
    }
    pub fn invalidate(&self) {
        self.live.store(false, std::sync::atomic::Ordering::Release);
    }
    pub fn is_live(&self) -> crate::application::file_dag::relations::types::Liveness {
        if self.live.load(std::sync::atomic::Ordering::Acquire) {
            crate::application::file_dag::relations::types::Liveness::Live
        } else {
            crate::application::file_dag::relations::types::Liveness::Released
        }
    }
}
