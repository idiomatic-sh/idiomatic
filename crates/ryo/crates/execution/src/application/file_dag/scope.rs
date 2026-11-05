//! Explicit outer scope creation over one required request schedule. Inner
//! operators/observation adapters receive its executor; they cannot mint owners.
use super::relations::RelationOwner;
use std::{rc::Rc, sync::Arc};

/// A permit's private field prevents unrelated application code from creating a
/// RelationOwner directly. Only this cohesive outer scope constructor mints it.
pub(super) struct ScopePermit {
    _private: (),
}
#[cfg(test)]
impl ScopePermit {
    pub(super) fn fixture() -> Self {
        Self { _private: () }
    }
}

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
    derived: Arc<dyn crate::outbound::derived::interface::DerivedPort>,
    stable: bool,
) -> Rc<RelationOwner> {
    Rc::new(RelationOwner::for_scope(
        ScopePermit { _private: () },
        stable,
        program
            .regions()
            .len()
            .saturating_add(ryo_relations::domain::relations::service::all().len())
            .saturating_add(1),
        derived,
    ))
}
