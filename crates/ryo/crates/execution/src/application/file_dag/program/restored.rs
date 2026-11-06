//! Engine-issued lazy restored input: immutable bytes retain their validated
//! root, coverage, partition set and owning generation until actual demand.
use super::admission::AdmittedInput;
use super::admission::Partition;
use super::admission::ProducerRoot;
use super::admission::Readiness;
use super::admission::RequiredPartitions;
use super::facade_runtime::types::PublicationScope;
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::application::file_dag::program::protocol::types::ObservedPublication;
use crate::domain::{
    completion::types::CoverageState,
    coverage::types::RuleIndex,
    executable_program::types::InputSource,
    operating::{errors::OperatingError, types::OperatorContext},
    operator::contract::views::BorrowedRows,
};
use ryo_values::domain::{
    plan_parts::types::InputNumber,
    stored_facts::types::{Bucket, KeptWords},
};
use std::sync::Arc;

pub(crate) mod types;
use types::ValidatedFrontier;

pub(in crate::application) fn validate_restored_frontier(
    prepared: &Arc<PreparedProgram>,
    producer: RuleIndex,
    input: InputNumber,
    first: Bucket,
    bucket: Bucket,
    required: &RequiredPartitions,
    payload: crate::outbound::facts::types::KeptBucket,
    scope: &PublicationScope,
) -> Result<ValidatedFrontier, OperatingError> {
    if !Arc::ptr_eq(prepared.program(), scope.program()) {
        return Err(OperatingError::NotRead);
    }
    scope.validate_partitions(required)?;
    let rule_id = prepared.rule_id(producer)?;
    for source in &payload.sources {
        let path = required
            .request
            .path(source.file)
            .ok_or(OperatingError::NotRead)?;
        if required
            .skipped
            .iter()
            .any(|skipped| &skipped.rule == rule_id && &skipped.path == path)
        {
            return Err(OperatingError::ObservationIncomplete { reason: "restored semantic frame belongs to a currently syntax-rejected producer partition".into() });
        }
    }
    let coverage = crate::application::file_dag::frontiers::service::validate_bucket(
        prepared, producer, input.0, first, bucket, required, &payload,
    )?;
    let source = *prepared
        .program()
        .rules()
        .get(producer.0)
        .and_then(|rule| rule.frontier.get(input.0))
        .ok_or(OperatingError::NotRun)?
        .as_ref()
        .map_err(Clone::clone)?;
    let kinds = match source {
        InputSource::Region(region) => prepared
            .program()
            .regions()
            .get(region.0)
            .and_then(|region| region.instructions.last())
            .ok_or(OperatingError::NotRun)?
            .contract
            .output_kinds
            .as_ref()
            .map_err(Clone::clone)?
            .clone(),
        InputSource::Relation(relation) => {
            ryo_relations::domain::relations::service::kinds(relation).to_vec()
        }
        InputSource::Observation(slot) => {
            let region = *prepared
                .program()
                .observation_regions()
                .get(slot)
                .ok_or(OperatingError::NotRun)?
                .as_ref()
                .map_err(Clone::clone)?;
            prepared
                .program()
                .regions()
                .get(region.0)
                .and_then(|region| region.instructions.last())
                .ok_or(OperatingError::NotRun)?
                .contract
                .output_kinds
                .as_ref()
                .map_err(Clone::clone)?
                .clone()
        }
        InputSource::Hosted { .. }
        | InputSource::ObservationMember { .. }
        | InputSource::EmptyObservation(_) => {
            return Err(OperatingError::WrongOperation);
        }
    };
    let root = match source {
        InputSource::Region(id) => ProducerRoot::Region(id),
        other => ProducerRoot::Input(other),
    };
    Ok(ValidatedFrontier {
        program: Arc::clone(prepared.program()),
        generation: scope.owner().generation_guard(),
        root,
        partition: required.clone(),
        coverage,
        kinds: kinds.into_boxed_slice(),
        payload: Arc::new(payload),
    })
}
impl ValidatedFrontier {
    pub(in crate::application) fn coverage(&self) -> &CoverageState {
        &self.coverage
    }

    pub(in crate::application) fn words(&self) -> &KeptWords {
        &self.payload.words
    }
    pub(super) fn materialize(
        &self,
        scope: &PublicationScope,
        required: &RequiredPartitions,
        transport: &ryo_language::domain::symbols::types::TransportPermit,
        context: &mut OperatorContext<'_>,
    ) -> Result<ObservedPublication, OperatingError> {
        if !Arc::ptr_eq(&self.program, scope.program())
            || !scope.owner().matches_generation(&self.generation)
            || !self.partition.same_request(required)
        {
            return Err(OperatingError::NotRead);
        }
        scope.validate_query(&context.symbols)?;
        let places = ryo_language::domain::symbols::service::query_places_mut(
            &mut context.symbols,
            transport,
        )
        .map_err(|_| OperatingError::NotRead)?;
        let before = places.len();
        let result = (|| {
            let table = ryo_values::domain::table_words::service::decode_complete(
                &self.payload.words.0,
                places,
            )
            .map_err(|error| OperatingError::ObservationIncomplete {
                reason: error.to_string(),
            })?;
            if table.kinds.as_slice() != self.kinds.as_ref() {
                return Err(OperatingError::ShapeMismatch);
            }
            let publication = scope.owner().computed(table)?;
            let unread = |_| Err(OperatingError::NotRead);
            let rows = scope.owner().resolve(publication.as_ref(), &unread)?;
            let proof = scope
                .owner()
                .admit(publication.as_ref(), BorrowedRows::from(&rows))?;
            AdmittedInput::from_publication(
                std::rc::Rc::clone(scope.owner()),
                publication.clone(),
                &proof,
                self.root,
                Partition::Project(self.partition.clone()),
                Readiness::Complete,
            )
            .map(|admitted| admitted.with_coverage(self.coverage.clone()))
        })();
        if result.is_err() {
            places.truncate(before);
        }
        let admitted = result?;
        Ok(ObservedPublication::from_admitted(admitted))
    }
}
