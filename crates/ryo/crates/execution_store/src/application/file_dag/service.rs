//! The store's one operation: explicit outer scope creation over one required request schedule, which opens a file
//! generation's relation owner. Inner operators/observation adapters receive its executor; they cannot mint owners.
use super::relations::{
    RelationOwner,
    types::{PinnedViewCount, ReuseEligibility},
};
pub(crate) mod types;
use std::{rc::Rc, sync::Arc};
use types::ScopePermit;

/// The partial facts a scope evaluates and publishes under: a file's, or the project's.
#[derive(Debug)]
pub enum ScopePartials<'a> {
    /// A file scope.
    File {
        /// The partials of the evaluation.
        evaluation: &'a [ryo_execution_domain::domain::coverage::types::Partial],
        /// The partials of the publication.
        publication: &'a [ryo_execution_domain::domain::coverage::types::Partial],
    },
    /// A project scope.
    Project {
        /// The partials of the evaluation.
        evaluation: &'a ryo_execution_domain::domain::coverage::types::Partials,
        /// The partials of the publication.
        publication: &'a ryo_execution_domain::domain::coverage::types::Partials,
    },
}

/// The one outer owner factory. Kernels have no way to acquire this permit.
pub fn open_program_owner(
    producers: crate::application::file_dag::relations::types::ProducerCapacity,
    derived: impl Into<crate::application::file_dag::derived::owner::types::DerivedAccess>,
    reuse: ReuseEligibility,
    routes: crate::application::file_dag::route_counts::profile::service::RouteProfile,
) -> Rc<RelationOwner> {
    Rc::new(RelationOwner::for_scope(
        ScopePermit { reuse },
        PinnedViewCount(
            producers
                .0
                .saturating_add(ryo_relations::domain::relations::service::all().len())
                .saturating_add(1),
        ),
        derived,
        routes,
    ))
}
