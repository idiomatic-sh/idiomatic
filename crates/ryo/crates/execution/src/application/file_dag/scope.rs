//! Explicit outer scope creation over one required request schedule. Inner
//! operators/observation adapters receive its executor; they cannot mint owners.
use super::relations::{
    RelationOwner,
    types::{PinnedViewCount, ReuseEligibility},
};
pub(in crate::application::file_dag) mod types;
use std::{rc::Rc, sync::Arc};
use types::ScopePermit;

pub(crate) enum ScopePartials<'a> {
    File {
        evaluation: &'a [crate::domain::coverage::types::Partial],
        publication: &'a [crate::domain::coverage::types::Partial],
    },
    Project {
        evaluation: &'a crate::domain::coverage::types::Partials,
        publication: &'a crate::domain::coverage::types::Partials,
    },
}

/// The one outer owner factory. Kernels have no way to acquire this permit.
pub(in crate::application::file_dag) fn open_program_owner(
    program: &Arc<crate::domain::executable_program::types::ExecutableProgram>,
    derived: impl Into<crate::application::file_dag::derived::owner::types::DerivedAccess>,
    reuse: ReuseEligibility,
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
    ))
}
