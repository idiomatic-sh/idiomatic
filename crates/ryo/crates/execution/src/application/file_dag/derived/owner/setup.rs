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
        let mut backend =
            super::super::indexes::new(&wanted, partial, super::super::indexes::LIMIT);
        if let Some(cache) = backend.as_mut() {
            cache.graph_enabled = prepared
                .graph_rules
                .iter()
                .any(|rule| demanded.binary_search(rule).is_ok());
            super::super::selection::activate_program(cache, prepared, demanded);
        }
        *self.backend.borrow_mut() = backend;
        self.file_prepared.set(true);
        Ok(())
    }
}
