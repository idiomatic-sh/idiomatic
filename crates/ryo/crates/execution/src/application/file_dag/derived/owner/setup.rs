//! File setup carries request metadata; demanded representation admission remains separate.
use super::super::prepared::PreparedDerived;
use super::*;
use crate::domain::coverage::types::Partial;
impl DerivedOwner {
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

impl DerivedOwner {
    pub(in crate::application::file_dag) fn prepare_program_file(
        &self,
        prepared: &PreparedDerived,
        demanded: &[crate::domain::coverage::types::RuleIndex],
        partial: &[Partial],
    ) -> Result<(), OperatingError> {
        if self.file_prepared.get() {
            return Err(OperatingError::NotRun);
        }
        let wanted = prepared.wanted(demanded)?;
        let mut backend = crate::application::file_dag::derived::owner::index::service::new(
            &wanted,
            partial,
            crate::application::file_dag::derived::owner::index::service::LIMIT,
        );
        if let Some(cache) = backend.as_mut() {
            // Request-wide graph eligibility is indexed once. File activation
            // examines only its demanded roots, never the full graph-rule inventory.
            cache.graph_enabled = demanded
                .iter()
                .any(|rule| prepared.graph_rules.contains(rule));
            crate::application::file_dag::derived::owner::selection::activate_program(
                cache, prepared, demanded,
            );
        }
        *self.backend.borrow_mut() = backend;
        self.file_prepared.set(true);
        Ok(())
    }
}
