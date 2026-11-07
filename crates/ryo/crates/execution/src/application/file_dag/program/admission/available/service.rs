//! Source availability: a published input or a first-borrow source inspected once.
use super::super::{
    AdmittedInput, AvailableBody, AvailableInput, Partition, ProducerRoot, ReadProof,
};
use crate::application::file_dag::relations::{RelationOwner, handle::service::PublishedInput};
use crate::domain::operating::errors::OperatingError;
use std::rc::Rc;

impl AvailableInput {
    /// Attach the demand's physical evidence without re-reading the cached
    /// source or forcing its deferred admission/geometry refusal.
    pub(in crate::application::file_dag::program) fn with_coverage(
        self,
        coverage: crate::domain::completion::types::CoverageState,
    ) -> Self {
        let body = match self.body {
            AvailableBody::Published(input) => {
                AvailableBody::Published(input.with_coverage(coverage))
            }
            AvailableBody::Source {
                owner,
                publication,
                partition,
                admission,
            } => AvailableBody::Source {
                owner,
                publication,
                partition,
                admission: admission.map(|input| input.with_coverage(coverage)),
            },
        };
        Self { body }
    }
    pub(in crate::application::file_dag::program) fn published(input: AdmittedInput) -> Self {
        Self {
            body: AvailableBody::Published(input),
        }
    }
    pub(in crate::application::file_dag::program) fn source(
        owner: Rc<RelationOwner>,
        publication: PublishedInput,
        root: ProducerRoot,
        partition: Partition,
        coverage: crate::domain::completion::types::CoverageState,
        rows: crate::domain::table_model::types::Rows<'_>,
    ) -> Result<Self, OperatingError> {
        // Inspect this exact first provider borrow now, not a later re-read.
        // Only geometry refusals are deferred; identity/generation/source failures
        // remain immediate. No borrowed rows survive this synchronous boundary.
        let inspected = owner.inspect_available(
            publication.as_ref(),
            crate::domain::table_model::types::RepresentationInput::from(&rows),
        )?;
        let admission = match inspected {
            Ok(physical) => {
                let physical = if physical.physical_shape() == crate::domain::table_model::collections::representations::types::PhysicalShape::ReadablePhysicalFallback {
                    // Freeze the readable prefix from this first supplied borrow in
                    // the existing generation-owned normalization record. Its refusal
                    // remains deferred to the declared admission boundary.
                    drop(physical);
                    owner.normalize(&publication, rows).and_then(|normalized| {
                        owner.admit(publication.as_ref(), crate::domain::table_model::types::RepresentationInput::from(&normalized))
                            .and_then(|proof| AdmittedInput::from_publication(Rc::clone(&owner), publication.clone(), &proof,
                                root, ReadProof::complete(partition.clone())))
                    })
                } else {
                    AdmittedInput::from_publication(Rc::clone(&owner), publication.clone(), &physical,
                        root, ReadProof::complete(partition.clone()))
                };
                physical.map(|input| input.with_coverage(coverage))
            }
            Err(geometry) => Err(geometry),
        };
        Ok(Self {
            body: AvailableBody::Source {
                owner,
                publication,
                partition,
                admission,
            },
        })
    }
    pub(in crate::application::file_dag::program) fn admit(
        &self,
        owner: &Rc<RelationOwner>,
        partition: &Partition,
    ) -> Result<AdmittedInput, OperatingError> {
        match &self.body {
            AvailableBody::Published(input) => {
                input.publication(owner)?;
                input.partition.require_supply(partition)?;
                Ok(input.clone())
            }
            AvailableBody::Source {
                owner: original,
                publication,
                partition: actual,
                admission,
            } => {
                if !Rc::ptr_eq(original, owner) {
                    return Err(OperatingError::NotRead);
                }
                actual.require_supply(partition)?;
                // Revalidate owner generation without obtaining another provider population.
                let _kinds = owner.publication_kinds(publication)?;
                admission.clone()
            }
        }
    }
}
