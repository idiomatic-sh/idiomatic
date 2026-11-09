//! Private revocation state; consumers can only observe liveness.
/// Read-only revocation witness, and the generation itself: the owner issues one lease per resident population, so
/// the lease's allocation is that population's generation. Only the population owner can invalidate it; semantic
/// callbacks never receive a mutable atomic or a revival operation.
#[derive(Debug)]
pub(crate) struct GenerationLease {
    live: std::sync::atomic::AtomicBool,
}
impl GenerationLease {
    pub(crate) fn new() -> Self {
        Self {
            live: std::sync::atomic::AtomicBool::new(true),
        }
    }
    pub(crate) fn invalidate(&self) {
        self.live.store(false, std::sync::atomic::Ordering::Release);
    }
    /// Whether two leases are one generation. The owner issues a fresh lease for each population and every proof
    /// keeps the lease it was issued under, so no released allocation can be reused for a later generation.
    pub(crate) fn same(
        this: &std::sync::Arc<Self>,
        other: &std::sync::Arc<Self>,
    ) -> crate::application::file_dag::relations::types::Sameness {
        if std::sync::Arc::ptr_eq(this, other) {
            crate::application::file_dag::relations::types::Sameness::Same
        } else {
            crate::application::file_dag::relations::types::Sameness::Different
        }
    }
    /// Whether the generation is still live.
    pub(crate) fn is_live(&self) -> crate::application::file_dag::relations::types::Liveness {
        if self.live.load(std::sync::atomic::Ordering::Acquire) {
            crate::application::file_dag::relations::types::Liveness::Live
        } else {
            crate::application::file_dag::relations::types::Liveness::Released
        }
    }
}
impl crate::application::file_dag::relations::types::Generation {
    /// Whether `other` is this generation.
    ///
    /// # Errors
    ///
    /// `NotRead` when it is another generation.
    pub fn same(
        &self,
        other: &Self,
    ) -> Result<(), ryo_execution_domain::domain::operating::errors::OperatingError> {
        match GenerationLease::same(&self.0, &other.0) {
            crate::application::file_dag::relations::types::Sameness::Same => Ok(()),
            crate::application::file_dag::relations::types::Sameness::Different => {
                Err(ryo_execution_domain::domain::operating::errors::OperatingError::NotRead)
            }
        }
    }
    /// The generation's address, for diagnostics to correlate events of one generation; never an authority.
    #[must_use]
    pub fn diagnostic_address(&self) -> *const () {
        std::sync::Arc::as_ptr(&self.0).cast::<()>()
    }
    /// Whether the generation is still live.
    ///
    /// # Errors
    ///
    /// `NotRead` once the owner released the population.
    pub fn live(&self) -> Result<(), ryo_execution_domain::domain::operating::errors::OperatingError> {
        match self.0.is_live() {
            crate::application::file_dag::relations::types::Liveness::Live => Ok(()),
            crate::application::file_dag::relations::types::Liveness::Released => {
                Err(ryo_execution_domain::domain::operating::errors::OperatingError::NotRead)
            }
        }
    }
}
