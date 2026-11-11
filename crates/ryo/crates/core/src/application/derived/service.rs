//! The derived owner's access as the core holds it: the edge composes it, the lint use case keeps it, and each run's engine prepares
//! with a clone of it. This is the one place in the core that names the access.

use ryo_execution::inbound::use_cases::prepare::interface::DerivedAccess;

/// The derived owner's access, held between the edge that composes it and the engine each run prepares.
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
pub struct Derived(DerivedAccess);

impl Derived {
    /// The access an engine prepares with: a clone of the one held.
    #[must_use]
    pub fn access(&self) -> DerivedAccess {
        self.0.clone()
    }
}

impl From<DerivedAccess> for Derived {
    fn from(access: DerivedAccess) -> Self {
        Self(access)
    }
}
