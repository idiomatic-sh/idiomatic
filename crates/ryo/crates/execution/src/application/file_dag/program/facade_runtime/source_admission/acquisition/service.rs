//! Demand-triggered source supply for one region input.
use super::super::SourceInputs;
use crate::application::file_dag::program::admission::AdmittedInput;
use crate::application::file_dag::program::admission::AvailableInput;
use crate::application::file_dag::program::admission::Inputs;
use crate::application::file_dag::program::admission::Partition;
use crate::application::file_dag::program::admission::ProducerRoot;
use crate::application::file_dag::program::admission::ReadProof;
use crate::application::file_dag::program::constants::{
    OBSERVATION_UNAVAILABLE, PRODUCER_COVERAGE_INCOMPLETE, PRODUCER_ROOT_INCOMPLETE,
    PRODUCER_WITHOUT_QUERY, SOURCE_FACTS_INCOMPLETE,
};
use crate::application::file_dag::program::facade_runtime::types::ObservationKey;
use crate::application::file_dag::relations::RelationOwner;
use crate::application::file_dag::relations::handle::service::PublishedInput;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use std::rc::Rc;

impl SourceInputs<'_, '_> {
    pub(in crate::application::file_dag::program::facade_runtime::source_admission) fn supply_source(
        &mut self,
        source: crate::domain::executable_program::types::InputSource,
        partition: &Partition,
        context: &mut OperatorContext<'_>,
    ) -> Result<AvailableInput, OperatingError> {
        use crate::domain::executable_program::types::InputSource;
        if let Some(project) = &mut self.project {
            if let Some(provided) = project.supply(source, partition, context, &self.owner) {
                return provided.map(|input| match source {
                    // Shared project facts need the same physical certificate
                    // as a direct provider, before any operator can re-key them.
                    InputSource::Relation(relation) => {
                        input.with_coverage(self.completion.relation_coverage(relation))
                    }
                    _ => input,
                });
            }
        }
        let admitted = match source {
            InputSource::Relation(relation) => {
                let actual_partition = self
                    .execution
                    .source_partition(ProducerRoot::Input(source))?;
                actual_partition.require_supply(partition)?;
                let rows = (self.read)(relation)?;
                let publication = self.owner.source(relation, &rows)?;
                return AvailableInput::source(
                    Rc::clone(&self.owner),
                    publication,
                    ProducerRoot::Input(source),
                    actual_partition,
                    self.completion.relation_coverage(relation),
                    rows,
                );
            }
            InputSource::Hosted { rule, relation } => {
                let actual_partition = self
                    .execution
                    .source_partition(ProducerRoot::Input(source))?;
                actual_partition.require_supply(partition)?;
                // The compiler owns the request-source declaration. Admission uses
                // the immutable prepared snapshot and the current numbered corpus.
                self.program.rule_metadata(rule)?;
                let hosted = self
                    .program
                    .hosted()
                    .get(relation.0 as usize)
                    .ok_or(OperatingError::NotRun)?;
                let table = crate::domain::hosted::service::table_of_query(
                    hosted,
                    crate::domain::hosted::types::NumberedFiles(context.paths),
                    &mut context.symbols,
                    context.phase,
                );
                let publication = self.owner.computed(table)?;
                admit_published(&self.owner, publication, source, &actual_partition)
            }
            InputSource::Region(id) => self
                .execution
                .published(self.state, id)?
                .ok_or(OperatingError::NotRun),
            InputSource::Observation(crate::domain::executable_program::types::ObservationPlace(slot)) => {
                if let Some(input) = self.observations.get(&ObservationKey(slot)) {
                    return Ok(AvailableInput::published(input.clone()));
                }
                let union = *self
                    .program
                    .program()
                    .observation_regions()
                    .get(slot)
                    .ok_or(OperatingError::NotRun)?
                    .as_ref()
                    .map_err(Clone::clone)?;
                let input = self
                    .execution
                    .published(self.state, union)?
                    .ok_or(OperatingError::NotRun)?;
                let admitted = crate::application::file_dag::program::admission::alias_observation(
                    self.program.program(),
                    crate::application::file_dag::program::admission::AliasSlot(slot),
                    &input,
                    &self.owner,
                )?;
                self.observations
                    .insert(ObservationKey(slot), admitted.clone());
                Ok(admitted)
            }
            InputSource::EmptyObservation(crate::domain::executable_program::types::ObservationPlace(slot)) => {
                let named = self
                    .program
                    .observation_metadata()
                    .ok_or(OperatingError::NotRun)?;
                let declaration = named.slots.get(slot).ok_or(OperatingError::NotRun)?;
                if declaration.rule != self.rule {
                    return Err(OperatingError::NotRead);
                }
                let binding = named
                    .bindings
                    .get(declaration.rule.0)
                    .and_then(|bindings| bindings.get(declaration.input.0 as usize))
                    .ok_or(OperatingError::NotRun)?;
                if !binding.producers.is_empty() {
                    return Err(OperatingError::NotRead);
                }
                let actual_partition = self
                    .execution
                    .source_partition(ProducerRoot::Input(source))?;
                actual_partition.require_supply(partition)?;
                let publication = self
                    .owner
                    .computed(ryo_values::domain::rows::types::Table {
                        kinds: binding
                            .schema
                            .columns
                            .iter()
                            .map(|column| column.kind)
                            .collect(),
                        count: ryo_values::domain::rows::types::RowCount(0),
                        cells: vec![],
                    })?;
                admit_published(&self.owner, publication, source, &actual_partition)
            }
            InputSource::ObservationMember {
                slot: crate::domain::executable_program::types::ObservationPlace(slot),
                member: crate::domain::executable_program::types::ProducerMember(member),
            } => {
                let outputs =
                    self.file_outputs
                        .ok_or_else(|| OperatingError::ObservationIncomplete {
                            reason: OBSERVATION_UNAVAILABLE.into(),
                        })?;
                let named = self
                    .program
                    .observation_metadata()
                    .ok_or(OperatingError::NotRun)?;
                let declaration = named.slots.get(slot).ok_or(OperatingError::NotRun)?;
                if declaration.rule != self.rule {
                    return Err(OperatingError::NotRead);
                }
                let producer = named
                    .bindings
                    .get(declaration.rule.0)
                    .and_then(|bindings| bindings.get(declaration.input.0 as usize))
                    .and_then(|binding| binding.producers.get(member))
                    .ok_or(OperatingError::NotRun)?;
                let crate::domain::file_program::types::ResolvedObservationProducer::Output {
                    rule,
                    output,
                } = producer
                else {
                    return Err(OperatingError::ObservationIncomplete {
                        reason: PRODUCER_WITHOUT_QUERY.into(),
                    });
                };
                let published = outputs
                    .get(&rule)
                    .and_then(|roots| roots.get(output.0 as usize))
                    .and_then(|root| root.publication.clone());
                if published.is_none()
                    && outputs
                        .get(&rule)
                        .and_then(|roots| roots.get(output.0 as usize))
                        .is_some_and(|root| root.readiness.is_some())
                {
                    return Err(OperatingError::ObservationIncomplete {
                        reason: PRODUCER_ROOT_INCOMPLETE.into(),
                    });
                }
                let admitted = match published {
                    Some(input) => input,
                    None => {
                        let metadata = self.program.rule_metadata(rule)?;
                        let root = metadata
                            .observations
                            .get(output.0 as usize)
                            .ok_or(OperatingError::NotRun)?;
                        if self
                            .completion
                            .for_rule(rule)
                            .observation_admission(&root.reads)
                            != crate::domain::completion::types::ObservationAdmission::Ready
                        {
                            return Err(OperatingError::ObservationIncomplete {
                                reason: SOURCE_FACTS_INCOMPLETE.into(),
                            });
                        }
                        let source = *root.source.as_ref().map_err(Clone::clone)?;
                        let previous = self.rule;
                        self.bind_rule(rule);
                        let admitted = self.admit_source(source, partition, context);
                        self.bind_rule(previous);
                        admitted?
                    }
                };
                if admitted.observation_ready()? == crate::application::file_dag::program::admission::ObservationReadiness::Waiting {
                    return Err(OperatingError::ObservationIncomplete {
                        reason: PRODUCER_COVERAGE_INCOMPLETE.into(),
                    });
                }
                crate::application::file_dag::program::admission::alias_member(
                    self.program,
                    crate::application::file_dag::program::admission::AliasSlot(slot),
                    crate::application::file_dag::program::admission::AliasMember(member),
                    &admitted,
                    partition,
                    &self.owner,
                )
            }
        }?;
        Ok(AvailableInput::published(admitted))
    }
}
pub(in crate::application::file_dag::program::facade_runtime::source_admission) fn admit_published(
    owner: &Rc<RelationOwner>,
    publication: PublishedInput,
    source: crate::domain::executable_program::types::InputSource,
    partition: &Partition,
) -> Result<AdmittedInput, OperatingError> {
    let unavailable = |_| Err(OperatingError::NotRead);
    let rows = owner.resolve(publication.as_ref(), &unavailable)?;
    let checked = owner.admit(
        publication.as_ref(),
        crate::domain::table_model::types::RepresentationInput::from(&rows),
    )?;
    AdmittedInput::from_publication(
        Rc::clone(owner),
        publication.clone(),
        &checked,
        match source {
            crate::domain::executable_program::types::InputSource::Region(id) => {
                ProducerRoot::Region(id)
            }
            _ => ProducerRoot::Input(source),
        },
        ReadProof::complete(partition.clone()),
    )
}
