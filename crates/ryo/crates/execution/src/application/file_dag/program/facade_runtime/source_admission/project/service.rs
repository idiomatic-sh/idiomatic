//! Project-wide source lookup for the project population.
use super::super::{ProjectSources, observation_slot};
use crate::application::file_dag::program::admission::AvailableInput;
use crate::application::file_dag::program::admission::Partition;
use crate::application::file_dag::program::constants::BOUND_PRODUCER_UNAVAILABLE;
use crate::application::file_dag::program::facade_runtime::types::{CachedInput, OrdinaryPlace, Provision};
use ryo_execution_store::application::file_dag::relations::RelationOwner;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use std::rc::Rc;

impl ProjectSources<'_> {
    pub(in crate::application::file_dag::program::facade_runtime::source_admission) fn ordinary_place(
        &self,
        source: crate::domain::executable_program::types::InputSource,
    ) -> Option<OrdinaryPlace> {
        self.ordinary
            .iter()
            .position(|input| matches!(input.source(), Ok(actual) if *actual == source))
            .map(OrdinaryPlace)
    }

    pub(in crate::application::file_dag::program::facade_runtime::source_admission) fn provides(
        &self,
        source: crate::domain::executable_program::types::InputSource,
    ) -> Provision {
        if matches!(
            source,
            crate::domain::executable_program::types::InputSource::ObservationMember { .. }
        ) || self.cache.iter().any(|cached| cached.source == source)
            || self.ordinary_place(source).is_some()
            || self.refused.iter().any(|refused| {
                self.sources.iter().any(|binding| {
                    binding.input == refused.input
                        && matches!(&binding.root, Ok(actual) if *actual == source)
                })
            })
        {
            Provision::Provided
        } else {
            Provision::Absent
        }
    }
    /// Called only by an actual DemandAction::Admit. Raw source handles and
    /// errors remain untouched when a branch does not demand them.
    pub(in crate::application::file_dag::program::facade_runtime::source_admission) fn supply(
        &mut self,
        source: crate::domain::executable_program::types::InputSource,
        partition: &Partition,
        context: &mut OperatorContext<'_>,
        owner: &Rc<RelationOwner>,
    ) -> Option<Result<AvailableInput, OperatingError>> {
        use crate::domain::executable_program::types::InputSource;
        if let Some(cached) = self.cache.iter().find(|cached| cached.source == source) {
            return Some(Ok(cached.input.clone()));
        }
        let program = self.program;
        let rule = self.rule;
        if let Some(at) = self.ordinary_place(source) {
            let actual_partition = match self.publication.execution.get() {
                Some(execution) => execution.partition(),
                None => return Some(Err(OperatingError::NotRead)),
            };
            if let Err(error) = actual_partition.require_supply(partition) {
                return Some(Err(error));
            }
            let result = self.ordinary[at.0].supply(owner, actual_partition);
            if let Ok(input) = &result {
                self.cache.push(CachedInput {
                    source,
                    input: input.clone(),
                });
            }
            return Some(result);
        }
        for refused in self.refused {
            if self.sources.iter().any(|binding| {
                binding.input == refused.input
                    && matches!(&binding.root, Ok(actual) if *actual == source)
            }) {
                return Some(Err(refused.error.clone()));
            }
        }
        if let InputSource::ObservationMember {
                slot: crate::domain::executable_program::types::ObservationPlace(slot),
                member: crate::domain::executable_program::types::ProducerMember(member),
            } = source {
            for bound in &mut *self.bound {
                let Some(binding) = bound.as_mut() else {
                    continue;
                };
                if !matches!(observation_slot(program, rule, binding.input), Ok(found) if found.0 == slot)
                {
                    continue;
                }
                let result = (|| {
                    let proof = binding
                        .members
                        .as_ref()
                        .map_err(Clone::clone)?
                        .get(member)
                        .ok_or(OperatingError::NotRun)?;
                    let producer = match proof {
                        crate::application::file_dag::frontiers::types::BoundMember::Current(
                            actual,
                        ) => actual.clone(),
                        crate::application::file_dag::frontiers::types::BoundMember::Restored(
                            token,
                        ) => {
                            let Partition::Project(required) = partition else {
                                return Err(OperatingError::WrongOperation);
                            };
                            token.materialize(
                                self.publication,
                                required,
                                self.transport,
                                context,
                            )?
                        }
                    };
                    crate::application::file_dag::program::admission::alias_member(
                        program,
                        crate::application::file_dag::program::admission::AliasSlot(slot),
                        crate::application::file_dag::program::admission::AliasMember(member),
                        producer.admitted(),
                        partition,
                        owner,
                    )
                })();
                let result = result.map(AvailableInput::published);
                if let Ok(input) = &result {
                    self.cache.push(CachedInput {
                    source,
                    input: input.clone(),
                });
                }
                return Some(result);
            }
            return Some(Err(OperatingError::ObservationIncomplete {
                reason: BOUND_PRODUCER_UNAVAILABLE.into(),
            }));
        }
        None
    }
}
