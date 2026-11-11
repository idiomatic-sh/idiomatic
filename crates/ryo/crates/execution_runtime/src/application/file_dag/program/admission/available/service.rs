//! Source availability: a published input or a first-borrow source inspected once.
use super::super::{
    service::AdmittedInput, service::AvailableInput, service::ReadProof, types::AvailableBody,
    types::Partition, types::ProducerRoot,
};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_store::application::file_dag::relations::interface::RelationsPort as _;
use ryo_execution_store::application::file_dag::relations::{
    handle::service::Publication, service::RelationOwner,
};
use std::rc::Rc;

impl AvailableInput {
    /// Attach the demand's physical evidence without re-reading the cached
    /// source or forcing its deferred admission/geometry refusal.
    pub(crate) fn with_coverage(
        self,
        coverage: ryo_execution_domain::domain::completion::types::CoverageState,
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
    pub(crate) fn published(input: AdmittedInput) -> Self {
        Self {
            body: AvailableBody::Published(input),
        }
    }
    pub(crate) fn source(
        owner: Rc<RelationOwner>,
        publication: Publication,
        root: ProducerRoot,
        partition: Partition,
        coverage: ryo_execution_domain::domain::completion::types::CoverageState,
        rows: ryo_execution_domain::domain::table_model::types::Rows<'_>,
    ) -> Result<Self, OperatingError> {
        // Inspect this exact first provider borrow now, not a later re-read.
        // Only geometry refusals are deferred; identity/generation/source failures
        // remain immediate. No borrowed rows survive this synchronous boundary.
        let inspected = owner.admit(
            &publication,
            ryo_execution_domain::domain::table_model::types::RepresentationInput::from(&rows),
        )?;
        let admission = match inspected {
            Ok(physical) => {
                let physical = if physical.physical_shape() == ryo_execution_domain::domain::table_model::types::PhysicalShape::ReadablePhysicalFallback {
                    // Freeze the readable prefix from this first supplied borrow in
                    // the existing generation-owned normalization record. Its refusal
                    // remains deferred to the declared admission boundary.
                    drop(physical);
                    owner.normalize(&publication, rows).and_then(|normalized| {
                        owner.admit(&publication, ryo_execution_domain::domain::table_model::types::RepresentationInput::from(&normalized)).and_then(|admitted| admitted)
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
    pub(crate) fn admit(
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
                let _kinds = owner.kinds(publication)?;
                admission.clone()
            }
        }
    }
}
