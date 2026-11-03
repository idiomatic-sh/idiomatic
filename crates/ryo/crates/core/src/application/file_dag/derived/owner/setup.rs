//! File setup carries request metadata; demanded representation admission remains separate.
use super::*;
use crate::domain::{coverage::types::Partial, operator::types::PreparedPool};
use ryo_language::domain::relations::types::Relation;
use ryo_values::domain::plans::types::Plan;
impl DerivedOwner {
    pub(in crate::application::file_dag) fn prepare_file(
        &self,
        enabled: bool,
        wanted: &[Relation],
        partial: &[Partial],
        selectable: &[Relation],
        plans: &[Plan],
        pool: &PreparedPool,
        active: impl Fn(usize) -> bool + Copy,
    ) {
        let mut backend = enabled
            .then(|| super::super::indexes::new(wanted, partial, super::super::indexes::LIMIT))
            .flatten();
        if let Some(backend) = backend.as_mut() {
            super::super::graph_cache::service::reserve(backend, plans, pool, active);
            super::super::selection::activate(backend, pool, selectable, active);
        }
        *self.backend.borrow_mut() = backend;
    }
    /// Configure the real installed backend for required-versus-optional controls.
    #[cfg(test)]
    pub(in crate::application::file_dag) fn set_optional_limit(
        &self,
        limit: usize,
    ) -> Result<(), OperatingError> {
        let mut backend = self.backend.borrow_mut();
        let backend = backend.as_mut().ok_or(OperatingError::NotRun)?;
        backend.limit = limit;
        Ok(())
    }
}
