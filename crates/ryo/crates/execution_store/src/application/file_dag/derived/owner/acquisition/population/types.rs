//! What one population-scoped acquisition decides and reports, as named answers.
use crate::application::file_dag::derived::owner::acquisition::semantics::storage::DerivedStorage;

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
    pub optional: OptionalRetention,
}

/// Storage built for one finite invocation, which no cache retains.
pub(in crate::application::file_dag) struct InvocationStorage {
    pub storage: std::sync::Arc<DerivedStorage>,
    pub construction: Construction,
}

/// The stored relation a build reads, when the build is for a stored relation.
pub(in crate::application::file_dag) struct StoredColumns {
    pub relation: ryo_relations::domain::relations::types::Relation,
}
