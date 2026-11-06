//! File setup carries request metadata; demanded representation admission remains separate.
use super::super::prepared::PreparedDerived;
use super::*;
use crate::application::file_dag::derived::constants as activation;
use crate::domain::coverage::types::{Partial, RuleIndex};
use ryo_relations::domain::relations::types::Relation;
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
        tracing::trace!(target: activation::TARGET, event = activation::BEGIN);
        let wanted = prepared.wanted(demanded)?;
        let mut backend = crate::application::file_dag::derived::owner::index::service::new(
            &wanted,
            partial,
            crate::application::file_dag::derived::owner::index::service::LIMIT,
        );
        if let Some(cache) = backend.as_mut() {
            prepared.bind_views(cache);
            // Request-wide graph eligibility is indexed once. File activation
            // examines only its demanded roots, never the full graph-rule inventory.
            for rule in demanded {
                if prepared.graph_for(*rule)? {
                    cache.graph_enabled = true;
                    break;
                }
            }
            crate::application::file_dag::derived::owner::selection::preparation::service::activate_program(
                cache, prepared, demanded,
            )?;
        }
        *self.backend.borrow_mut() = backend;
        self.file_prepared.set(true);
        tracing::trace!(target: activation::TARGET, event = activation::END);
        Ok(())
    }
}

impl PreparedDerived {
    pub(in crate::application::file_dag) fn wanted(
        &self,
        demanded: &[RuleIndex],
    ) -> Result<Vec<Relation>, OperatingError> {
        if !demanded.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err(OperatingError::NotRun);
        }
        let mut wanted = Vec::new();
        for rule in demanded {
            let relations = self.relations_for(*rule)?;
            for relation in relations.iter() {
                tracing::trace!(target: activation::TARGET, event = activation::RELATION);
                if !wanted.contains(relation) {
                    wanted.push(*relation);
                }
            }
        }
        Ok(wanted)
    }
}
