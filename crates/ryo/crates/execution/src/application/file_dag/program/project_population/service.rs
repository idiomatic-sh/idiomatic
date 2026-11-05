//! Retain each structurally equal source once; row admission remains demand-driven.
use super::super::{
    admission::{AvailableInput, Partition, ProducerRoot},
    facade_runtime::PublicationScope,
};
use crate::application::file_dag::{
    derived::owner::PopulationId,
    frontiers::types::{BoundInput, FrontierInput, FrontierSource},
    relations::RelationOwner,
};
use crate::domain::{
    completion::types::{CoverageState, completion_policy::value::CompletionPolicy},
    executable_program::types::InputSource,
    operating::errors::OperatingError,
};
use ryo_values::domain::{plan_parts::types::Input, rows::types::Table};
use std::{cell::OnceCell, rc::Rc, sync::Arc};

#[derive(PartialEq, Eq)]
enum Payload {
    Flat(Arc<Table>),
    Compact(Arc<ryo_values::domain::table_words::types::CompactTable>),
}
/// The ID and backing are inseparable and privately issued by this phase owner.
pub(in crate::application::file_dag::program) struct Population {
    id: PopulationId,
    source: Result<InputSource, OperatingError>,
    coverage: CoverageState,
    payload: Payload,
    admitted: OnceCell<Result<AvailableInput, OperatingError>>,
}
impl Population {
    pub(in crate::application::file_dag::program) fn provides(&self, source: InputSource) -> bool {
        self.source.as_ref().is_ok_and(|actual| *actual == source)
    }
    pub(in crate::application::file_dag::program) fn supply(
        &self,
        owner: &Rc<RelationOwner>,
        partition: &Partition,
    ) -> Result<AvailableInput, OperatingError> {
        self.admitted
            .get_or_init(|| {
                let source = self.source.clone()?;
                let publication = match &self.payload {
                    Payload::Flat(table) => owner.source_shared_computed(Arc::clone(table)),
                    Payload::Compact(table) => owner.source_compact(Arc::clone(table)),
                }?;
                let unavailable = |_| Err(OperatingError::NotRead);
                let rows = owner.resolve(publication.as_ref(), &unavailable)?;
                AvailableInput::source(
                    Rc::clone(owner),
                    publication.clone(),
                    match source {
                        InputSource::Region(id) => ProducerRoot::Region(id),
                        _ => ProducerRoot::Input(source),
                    },
                    partition.clone(),
                    self.coverage.clone(),
                    rows,
                )
            })
            .clone()
    }
}
/// No owner or evaluator is allocated per population: these are input and slot identities.
#[derive(Default)]
pub(in crate::application::file_dag::program) struct Populations {
    sources: Vec<Rc<Population>>,
    invocations: Vec<(InvocationKey, PopulationId)>,
}
#[derive(PartialEq, Eq)]
struct InvocationKey {
    sources: Vec<PopulationId>,
    refused: Vec<(Input, OperatingError)>,
    policy: CompletionPolicy,
    bound: Vec<BoundKey>,
}
#[derive(PartialEq, Eq)]
enum BoundKey {
    Current(PopulationId, CoverageState),
    Restored(super::super::restored::ValidatedFrontier),
    Refused(OperatingError),
    Input(ryo_values::domain::observations::types::ObservationBindingId),
}
impl Populations {
    pub(in crate::application::file_dag::program) fn sources(
        &mut self,
        owner: &Rc<RelationOwner>,
        sources: &[FrontierSource],
        rows: Vec<FrontierInput>,
    ) -> Result<Vec<Rc<Population>>, OperatingError> {
        rows.into_iter()
            .map(|row| {
                let (input, coverage, payload) = match row {
                    FrontierInput::Decoded { frontier, coverage } => (
                        frontier.input,
                        coverage,
                        Payload::Flat(Arc::new(frontier.table)),
                    ),
                    FrontierInput::Compact {
                        input,
                        table,
                        coverage,
                        ..
                    } => (input, coverage, Payload::Compact(Arc::new(table))),
                };
                let source = sources
                    .iter()
                    .find(|source| source.input == input)
                    .ok_or(OperatingError::NotRead)?
                    .root
                    .clone();
                if let Some(found) = self.sources.iter().find(|found| {
                    found.source == source && found.coverage == coverage && found.payload == payload
                }) {
                    return Ok(Rc::clone(found));
                }
                let entry = Rc::new(Population {
                    id: owner.project_identity()?,
                    source,
                    coverage,
                    payload,
                    admitted: OnceCell::new(),
                });
                self.sources.push(Rc::clone(&entry));
                Ok(entry)
            })
            .collect()
    }
    pub(in crate::application::file_dag::program) fn invocation(
        &mut self,
        scope: &PublicationScope,
        sources: &[Rc<Population>],
        refused: &[crate::domain::crossing::types::RefusedFrontier],
        bound: &[Option<BoundInput>],
        policy: CompletionPolicy,
    ) -> Result<PopulationId, OperatingError> {
        let mut bindings = Vec::new();
        for binding in bound.iter().flatten() {
            bindings.push(BoundKey::Input(binding.input));
            match &binding.members {
                Err(error) => bindings.push(BoundKey::Refused(error.clone())),
                Ok(members) => {
                    for member in members {
                        bindings.push(match member {
                        crate::application::file_dag::frontiers::types::BoundMember::Current(publication) => BoundKey::Current(publication.admitted().population(scope.owner())?, publication.admitted().coverage()?),
                        crate::application::file_dag::frontiers::types::BoundMember::Restored(token) => BoundKey::Restored(token.clone()),
                    });
                    }
                }
            }
        }
        let key = InvocationKey {
            sources: sources.iter().map(|source| source.id).collect(),
            refused: refused
                .iter()
                .map(|entry| (entry.input, entry.error.clone()))
                .collect(),
            policy,
            bound: bindings,
        };
        if let Some((_, id)) = self.invocations.iter().find(|(prior, _)| *prior == key) {
            return Ok(*id);
        }
        let id = scope.owner().project_identity()?;
        self.invocations.push((key, id));
        Ok(id)
    }
}
