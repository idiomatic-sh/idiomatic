//! Test-only observation of producer lease release, route work and retry checkpoints.
#[cfg(test)]
use super::super::ProducerLeaseProbe;
#[cfg(test)]
use super::super::State;
use super::super::{ProgramExecution, RuntimeState};
#[cfg(test)]
use crate::application::file_dag::program::constants::LIFECYCLE_OBSERVER;
#[cfg(test)]
use crate::application::file_dag::program::constants::OBSERVED_PRODUCER;
#[cfg(test)]
use crate::application::file_dag::program::runtime::types::{
    AttemptCheckpoint, ConsumerReleasePoint, Dependencies, DependencyCheckpoint, IndexLiveness,
    ProducerLeaseEvent, ReaderCount, RetryCheckpoint, WorkingBytes,
};
use ryo_execution_domain::domain::executable_program::types::RegionId;
use ryo_execution_domain::domain::operating::errors::OperatingError;

impl ProgramExecution {
    /// Whether the file scope holds a slot, or a dormant reader count, for a region: a region never instantiated has neither.
    #[cfg(test)]
    pub(crate) fn instantiated(
        &self,
        id: RegionId,
    ) -> crate::application::file_dag::program::runtime::types::Instantiation {
        use crate::application::file_dag::program::runtime::types::{SlotHolding, Instantiation};
        let held = |present| if present { SlotHolding::Held } else { SlotHolding::Missing };
        Instantiation {
            slot: held(self.file_state.slots.borrow().contains_key(&id)),
            dormant: held(self.file_state.dormant.borrow().contains_key(&id)),
        }
    }
    /// Observe actual release boundaries without retaining a producer or a backing lease.
    #[cfg(test)]
    pub(crate) fn observe_producer_releases(
        &self,
        state: &RuntimeState,
        producer: RegionId,
        consumers: &[RegionId],
    ) -> Result<(), OperatingError> {
        self.validate_state(state)?;
        if producer.0 >= self.program.regions().len()
            || consumers
                .iter()
                .any(|id| id.0 >= self.program.regions().len())
            || self.producer_lease_probe.borrow().is_some()
        {
            return Err(OperatingError::NotRun);
        }
        *self.producer_lease_probe.borrow_mut() = Some(ProducerLeaseProbe {
            scope: state.scope,
            producer,
            consumers: consumers.into(),
            events: vec![],
        });
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn observe_consumer_release(
        &self,
        state: &RuntimeState,
        consumer: RegionId,
        point: ConsumerReleasePoint,
    ) {
        let mut probe = self.producer_lease_probe.borrow_mut();
        let Some(probe) = probe
            .as_mut()
            .filter(|probe| probe.scope == state.scope && probe.consumers.contains(&consumer))
        else {
            return;
        };
        let checkpoint = self
            .retry_checkpoint(state, probe.producer)
            .expect(OBSERVED_PRODUCER);
        let index = self
            .owner
            .producer_view_witness(
                state.scope,
                probe.producer,
                ryo_derived::domain::request::types::DerivedKind::Equality,
            )
            .unwrap();
        let index_liveness = if index.as_ref().is_some_and(|index| {
            index.liveness()
                == ryo_execution_store::application::file_dag::derived::owner::witness::service::BackingLiveness::Alive
        }) {
            IndexLiveness::Alive
        } else {
            IndexLiveness::Released
        };
        probe.events.push(ProducerLeaseEvent {
            consumer,
            point,
            checkpoint,
            working_bytes: WorkingBytes(self.owner.derived().working_bytes().0),
            index,
            index_liveness,
        });
    }
    #[cfg(test)]
    pub(crate) fn take_producer_releases(&self, state: &RuntimeState) -> Vec<ProducerLeaseEvent> {
        self.validate_state(state).unwrap();
        let probe = self
            .producer_lease_probe
            .borrow_mut()
            .take()
            .expect(LIFECYCLE_OBSERVER);
        assert_eq!(probe.scope, state.scope);
        probe.events
    }
    /// Observes completed dispatches even after the real retirement drops invocation scratch.
    #[cfg(test)]
    pub(crate) fn route_observation(
        &self,
        state: &RuntimeState,
        rule: ryo_execution_domain::domain::coverage::types::RuleIndex,
        step: ryo_values::domain::plan_parts::types::StepId,
    ) -> Result<
        ryo_execution_domain::domain::operating::fixtures::ExecutionObservation,
        OperatingError,
    > {
        let origin = self
            .program
            .rules()
            .get(rule.0)
            .and_then(|rule| rule.original_steps.get(step.0 as usize))
            .ok_or_else(|| OperatingError::NotRun)?
            .as_ref()
            .map_err(Clone::clone)?;
        let region = self
            .program
            .logical_regions()
            .get(origin.logical.0)
            .ok_or_else(|| OperatingError::NotRun)?;
        self.region_observation(state, *region)
    }
    #[cfg(test)]
    pub(crate) fn region_observation(
        &self,
        state: &RuntimeState,
        region: RegionId,
    ) -> Result<
        ryo_execution_domain::domain::operating::fixtures::ExecutionObservation,
        OperatingError,
    > {
        self.validate_state(state)?;
        let slots = state.slots.borrow();
        let Some(slot) = slots.get(&region) else {
            self.program
                .regions()
                .get(region.0)
                .ok_or_else(|| OperatingError::NotRun)?;
            return Ok(Default::default());
        };
        Ok(
            ryo_execution_domain::domain::operating::fixtures::ExecutionObservation {
                registered_calls: slot.registered_calls.0,
                ..slot.observed
            },
        )
    }
    #[cfg(test)]
    pub(crate) fn retry_checkpoint(
        &self,
        state: &RuntimeState,
        id: RegionId,
    ) -> Result<RetryCheckpoint, OperatingError> {
        self.validate_state(state)?;
        let slots = state.slots.borrow();
        let Some(slot) = slots.get(&id) else {
            self.program
                .regions()
                .get(id.0)
                .ok_or_else(|| OperatingError::NotRun)?;
            let remaining = match state.dormant.borrow().get(&id).copied() {
                Some(remaining) => remaining,
                None => state.initial_remaining(&self.program, id)?,
            };
            return Ok(RetryCheckpoint {
                attempt: if state.dormant.borrow().get(&id) == Some(&ReaderCount(0)) {
                    AttemptCheckpoint::Terminal
                } else {
                    AttemptCheckpoint::Dormant
                },
                remaining,
                dependencies: if state.dormant.borrow().get(&id) == Some(&ReaderCount(0)) {
                    DependencyCheckpoint::Consumed
                } else {
                    DependencyCheckpoint::Pending
                },
            });
        };
        Ok(RetryCheckpoint {
            attempt: match slot.state {
                State::Dormant => AttemptCheckpoint::Dormant,
                State::Running | State::Suspended | State::Ready => AttemptCheckpoint::Running,
                State::Published(_) => AttemptCheckpoint::Published,
                State::Retired | State::Failed(_) => AttemptCheckpoint::Terminal,
            },
            remaining: slot.remaining,
            dependencies: if slot.dependencies_consumed == Dependencies::Consumed {
                DependencyCheckpoint::Consumed
            } else {
                DependencyCheckpoint::Pending
            },
        })
    }
}
