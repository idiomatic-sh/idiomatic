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
/// The generation of a resident population: the lease the owner issued for it. A proof keeps it and presents it back.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::fixtures::program_derived_port;
/// use ryo_execution_store::application::file_dag::relations::{interface::RelationsPort, RelationOwner};
/// use ryo_execution_store::application::file_dag::relations::types::ReuseEligibility;
/// let owner = RelationOwner::fixture(ReuseEligibility::Eligible, program_derived_port());
/// let generation = owner.generation();
/// assert!(generation.same(&owner.generation()).is_ok());
/// ```
#[derive(Clone, Debug)]
pub struct ResidentGeneration(pub(crate) std::sync::Arc<GenerationLease>);
impl ResidentGeneration {
    /// Whether `other` is this generation.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_domain::domain::operating::errors::OperatingError;
    /// use ryo_execution_store::application::file_dag::fixtures::program_derived_port;
    /// use ryo_execution_store::application::file_dag::relations::{interface::RelationsPort, RelationOwner};
    /// use ryo_execution_store::application::file_dag::relations::types::ReuseEligibility;
    /// let a = RelationOwner::fixture(ReuseEligibility::Eligible, program_derived_port());
    /// let b = RelationOwner::fixture(ReuseEligibility::Eligible, program_derived_port());
    /// assert!(a.generation().same(&a.generation()).is_ok());
    /// assert!(matches!(a.generation().same(&b.generation()), Err(OperatingError::NotRead)));
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::fixtures::program_derived_port;
    /// use ryo_execution_store::application::file_dag::relations::{interface::RelationsPort, RelationOwner};
    /// use ryo_execution_store::application::file_dag::relations::types::ReuseEligibility;
    /// let owner = RelationOwner::fixture(ReuseEligibility::Eligible, program_derived_port());
    /// let generation = owner.generation();
    /// assert_eq!(generation.diagnostic_address(), generation.clone().diagnostic_address());
    /// ```
    #[must_use]
    pub fn diagnostic_address(&self) -> *const () {
        std::sync::Arc::as_ptr(&self.0).cast::<()>()
    }
    /// Whether the generation is still live.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::fixtures::program_derived_port;
    /// use ryo_execution_store::application::file_dag::relations::{interface::RelationsPort, RelationOwner};
    /// use ryo_execution_store::application::file_dag::relations::types::ReuseEligibility;
    /// let owner = RelationOwner::fixture(ReuseEligibility::Eligible, program_derived_port());
    /// let generation = owner.generation();
    /// assert!(generation.live().is_ok());
    /// owner.release();
    /// assert!(generation.live().is_err());
    /// ```
    ///
    /// # Errors
    ///
    /// `NotRead` once the owner released the population.
    pub fn live(
        &self,
    ) -> Result<(), ryo_execution_domain::domain::operating::errors::OperatingError> {
        match self.0.is_live() {
            crate::application::file_dag::relations::types::Liveness::Live => Ok(()),
            crate::application::file_dag::relations::types::Liveness::Released => {
                Err(ryo_execution_domain::domain::operating::errors::OperatingError::NotRead)
            }
        }
    }
}
