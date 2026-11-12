//! Execution's derived preparation as the core holds it: the edge composes it, the lint use case keeps it, and each run's
//! engine prepares with a clone of it. It lends no access and acquires no view.

use ryo_execution::application::file_dag::derived::service::DerivedPreparation;
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_execution_runtime::outbound::digest::interface::DigestPort;
use ryo_execution_runtime::outbound::operators::interface::OperatorsPort;
use std::sync::Arc;

/// Execution's derived preparation, held between the edge that composes it and the engine each run prepares.
///
/// # Examples
///
/// ```
/// use ryo_core::application::derived::service::Derived;
///
/// let derived = Derived::from(ryo_execution::derived());
/// let lent = derived.access();
/// ```
#[derive(Clone, Debug)]
pub struct Derived(DerivedPreparation);

impl Derived {
    /// The preparation an engine prepares with: a clone of the one held.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_core::application::derived::service::Derived;
    ///
    /// let derived = Derived::from(ryo_execution::derived());
    /// let first = derived.access();
    /// let second = derived.access();
    /// ```
    #[must_use]
    pub fn access(&self) -> DerivedPreparation {
        self.0.clone()
    }

    /// The engine's prepare use case over `operators` and `digest`, preparing with a clone of the held preparation.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_core::application::derived::service::Derived;
    /// use std::sync::Arc;
    ///
    /// let derived = Derived::from(ryo_execution::derived());
    /// let operators = ryo_execution_runtime::outbound::operators::factory::use_case(
    ///     ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(Arc::new(
    ///         ryo_operators::outbound::matching::factory::use_case(),
    ///     )),
    /// );
    /// let digest = ryo_execution_runtime::outbound::digest::factory::blake3();
    /// let prepare = derived.prepare(Arc::new(operators), Arc::new(digest));
    /// ```
    #[must_use]
    pub fn prepare(
        &self,
        operators: Arc<dyn OperatorsPort>,
        digest: Arc<dyn DigestPort>,
    ) -> PrepareUseCaseImpl {
        PrepareUseCaseImpl::new(operators, self.access(), digest)
    }
}

impl From<DerivedPreparation> for Derived {
    fn from(access: DerivedPreparation) -> Self {
        Self(access)
    }
}
