//! What one population-scoped acquisition decides and reports, as named answers.
use crate::application::file_dag::derived::owner::acquisition::semantics::service::DerivedStorage;

/// Whether the view was constructed by this acquisition or found already built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) enum Construction {
    Built,
    Reused,
}

/// Whether the acquisition observes a stored relation of the cache's eligible set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) enum StoredObservation {
    Observed,
    Unobserved,
}

/// Whether the cache backend already accounts the constructed bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) enum BackendRetention {
    Accounted,
    Unaccounted,
}

/// Whether the owner may retain the constructed view beyond its required holders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) enum OptionalRetention {
    Allowed,
    Refused,
}

/// A constructed or reused storage with the answers the owner needs to account it.
pub(in crate::application::file_dag) struct Acquisition<S = DerivedStorage> {
    pub storage: S,
    pub construction: Construction,
    pub backend: BackendRetention,
    pub optional: OptionalRetention,
}

/// Storage built for one finite invocation, which no cache retains.
pub(in crate::application::file_dag) struct InvocationStorage {
    pub storage: std::sync::Arc<DerivedStorage>,
    pub construction: Construction,
}
