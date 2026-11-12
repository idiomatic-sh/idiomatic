//! Opening a scope's relation owner, and building one over the derived access its scope was given.
use super::permit::service::ScopePermit;
use crate::application::file_dag::derived::owner::service::{DerivedAccess, DerivedOwner};
use crate::application::file_dag::derived::owner::types::EntryLimit;
use crate::application::file_dag::relations::service::RelationOwner;
use crate::application::file_dag::relations::types::{
    PinnedViewCount, ProducerCapacity, ReuseEligibility,
};
use crate::application::file_dag::route_counts::profile::service::RouteProfile;
use std::rc::Rc;

/// The one outer owner factory: a scope opens its file generation's relation owner over the derived access its opener
/// hands in. Kernels have no way to acquire the permit it mints.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::derived::relation_owner::service::open_program_owner;
/// use ryo_execution_store::application::file_dag::fixtures::program_derived_port;
/// use ryo_execution_store::application::file_dag::relations::types::{ProducerCapacity, ReuseEligibility};
///
/// let owner = open_program_owner(
///     ProducerCapacity(1),
///     program_derived_port(),
///     ReuseEligibility::Eligible,
///     Default::default(),
/// );
/// assert_eq!(owner.stable(), ReuseEligibility::Eligible);
/// ```
pub fn open_program_owner(
    producers: ProducerCapacity,
    derived: impl Into<DerivedAccess>,
    reuse: ReuseEligibility,
    routes: RouteProfile,
) -> Rc<RelationOwner> {
    Rc::new(RelationOwner::for_scope(
        ScopePermit { reuse },
        PinnedViewCount(
            producers
                .0
                .saturating_add(ryo_relations::domain::relations::service::all().len())
                .saturating_add(1),
        ),
        derived.into(),
        routes,
    ))
}

impl RelationOwner {
    fn for_scope(
        permit: ScopePermit,
        views: PinnedViewCount,
        semantic: DerivedAccess,
        routes: RouteProfile,
    ) -> Self {
        Self::with_limits(ReuseEligibility::from(permit), views, semantic, routes)
    }
    /// An owner over `semantic` with the default pin limit and no route recording, for the proofs of the store and the engine.
    #[cfg(any(test, feature = "fixtures"))]
    pub(crate) fn new(stable: ReuseEligibility, semantic: impl Into<DerivedAccess>) -> Self {
        Self::with_limits(stable, PinnedViewCount(1024), semantic, Default::default())
    }
    /// An owner whose work records into `routes`.
    #[cfg(test)]
    pub(crate) fn recording(
        stable: ReuseEligibility,
        semantic: impl Into<DerivedAccess>,
        routes: RouteProfile,
    ) -> Self {
        Self::with_limits(stable, PinnedViewCount(1024), semantic, routes)
    }
    #[cfg(any(test, feature = "fixtures"))]
    pub(crate) fn for_request(
        stable: ReuseEligibility,
        views: PinnedViewCount,
        semantic: impl Into<DerivedAccess>,
    ) -> Self {
        Self::with_limits(stable, views, semantic, Default::default())
    }
    fn with_limits(
        stable: ReuseEligibility,
        max_views: PinnedViewCount,
        semantic: impl Into<DerivedAccess>,
        routes: RouteProfile,
    ) -> Self {
        let max_derived = EntryLimit(
            max_views
                .0
                .saturating_mul(ryo_derived::domain::request::types::DerivedKind::ALL.len()),
        );
        Self::with_derived(
            stable,
            max_views,
            DerivedOwner::new(max_derived, semantic, routes.clone()),
            routes,
        )
    }
    /// A store over `derived` with the default pin limit and no route recording, for the proofs of the store and the engine.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::fixtures::program_derived_port;
    /// use ryo_execution_store::application::file_dag::relations::{types::ReuseEligibility, service::RelationOwner};
    /// let owner = RelationOwner::fixture(ReuseEligibility::Eligible, program_derived_port());
    /// assert_eq!(owner.stable(), ReuseEligibility::Eligible);
    /// ```
    #[cfg(any(test, feature = "fixtures"))]
    #[must_use]
    pub fn fixture(reuse: ReuseEligibility, derived: impl Into<DerivedAccess>) -> Self {
        Self::with_limits(reuse, PinnedViewCount(1024), derived, Default::default())
    }
}
