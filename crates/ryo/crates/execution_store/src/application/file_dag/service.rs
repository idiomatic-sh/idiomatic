//! The store's one operation: explicit outer scope creation over one required request schedule, which opens a file
//! generation's relation owner. Inner operators/observation adapters receive its executor; they cannot mint owners.
use super::relations::{
    RelationOwner,
    types::{PinnedViewCount, ReuseEligibility},
};
pub(in crate::application::file_dag) mod types;
use std::{rc::Rc, sync::Arc};
use types::ScopePermit;

/// The partial facts a scope evaluates and publishes under: a file's, or the project's.
pub enum ScopePartials<'a> {
    File {
        evaluation: &'a [ryo_execution_domain::domain::coverage::types::Partial],
        publication: &'a [ryo_execution_domain::domain::coverage::types::Partial],
    },
    Project {
        evaluation: &'a ryo_execution_domain::domain::coverage::types::Partials,
        publication: &'a ryo_execution_domain::domain::coverage::types::Partials,
    },
}

/// The one outer owner factory. Kernels have no way to acquire this permit.
pub fn open_program_owner(
    program: &Arc<ryo_execution_domain::domain::executable_program::types::executable_program::value::ExecutableProgram>,
    derived: impl Into<crate::application::file_dag::derived::owner::types::DerivedAccess>,
    reuse: ReuseEligibility,
    routes: crate::application::file_dag::route_counts::types::RouteProfile,
) -> Rc<RelationOwner> {
    Rc::new(RelationOwner::for_scope(
        ScopePermit { reuse },
        PinnedViewCount(
            program
                .regions()
                .len()
                .saturating_add(ryo_relations::domain::relations::service::all().len())
                .saturating_add(1),
        ),
        derived,
        routes,
    ))
}
