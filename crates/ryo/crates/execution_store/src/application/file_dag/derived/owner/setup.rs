//! File setup carries request metadata; demanded representation admission remains separate.
use super::super::prepared::RequestDerived;
use super::*;
use crate::application::file_dag::derived::constants as activation;
use crate::application::file_dag::derived::owner::cache::types::GraphAdmission;
use ryo_execution_domain::domain::coverage::types::{Partial, RuleIndex};
use ryo_relations::domain::relations::types::Relation;
impl DerivedOwner {
    /// Configure the real installed backend for required-versus-optional controls.
    #[cfg(any(test, feature = "fixtures"))]
    pub fn set_optional_limit(
        &self,
        limit: crate::application::file_dag::derived::owner::index::types::ByteLimit,
    ) -> Result<(), OperatingError> {
        let mut backend = self.backend.borrow_mut();
        let backend = backend.as_mut().ok_or(OperatingError::NotRun)?;
        backend.limit = limit;
        Ok(())
    }
}

impl DerivedOwner {
    /// Sets the owner up for one file of the program: the views its demanded rules read.
    pub fn prepare_program_file(
        &self,
        prepared: &RequestDerived,
        demanded: &[ryo_execution_domain::domain::coverage::types::RuleIndex],
        partial: &[Partial],
    ) -> Result<(), OperatingError> {
        if self.file_prepared.get() == FilePreparation::Prepared {
            return Err(OperatingError::NotRun);
        }
        tracing::trace!(target: activation::TARGET, event = activation::BEGIN);
        let wanted = prepared.wanted(demanded)?;
        let mut backend = crate::application::file_dag::derived::owner::index::service::new(
            &wanted,
            partial,
            crate::application::file_dag::derived::owner::index::types::ByteLimit(
                crate::application::file_dag::derived::owner::index::service::LIMIT,
            ),
        );
        if let Some(cache) = backend.as_mut() {
            // Request-wide graph eligibility is indexed once. File activation
            // examines only its demanded roots, never the full graph-rule inventory.
            for rule in demanded {
                if prepared.graph_for(*rule)? == GraphAdmission::Enabled {
                    cache.graph_enabled = crate::application::file_dag::derived::owner::cache::types::GraphAdmission::Enabled;
                    break;
                }
            }
        }
        *self.backend.borrow_mut() = backend;
        self.file_prepared.set(FilePreparation::Prepared);
        tracing::trace!(target: activation::TARGET, event = activation::END);
        Ok(())
    }
}

impl RequestDerived {
    pub(crate) fn wanted(
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
