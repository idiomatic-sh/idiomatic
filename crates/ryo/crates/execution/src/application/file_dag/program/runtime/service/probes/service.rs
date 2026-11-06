//! Test-only observation of producer lease release, route work and retry checkpoints.
#[cfg(test)]
use super::super::ProducerLeaseProbe;
use super::super::{ProgramExecution, RuntimeState, State};
#[cfg(test)]
use crate::application::file_dag::program::constants::LIFECYCLE_OBSERVER;
#[cfg(test)]
use crate::application::file_dag::program::constants::OBSERVED_PRODUCER;
use crate::application::file_dag::program::runtime::types::{
    AttemptCheckpoint, ConsumerReleasePoint, Dependencies, DependencyCheckpoint, IndexLiveness,
    ProducerLeaseEvent, ReaderCount, RetryCheckpoint, WorkingBytes,
};
use crate::domain::executable_program::types::RegionId;
use crate::domain::operating::errors::OperatingError;

impl ProgramExecution {
    /// Observe actual release boundaries without retaining a producer or a backing lease.
    #[cfg(test)]
    pub(in crate::application::file_dag::program) fn observe_producer_releases(
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
    pub(in crate::application::file_dag::program::runtime::service) fn observe_consumer_release(
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
                ryo_derived::domain::contract::types::DerivedKind::Equality,
            )
            .unwrap();
        let index_liveness = if index.as_ref().is_some_and(|index| {
            index.liveness()
                == crate::application::file_dag::derived::owner::witness::service::BackingLiveness::Alive
        }) {
            IndexLiveness::Alive
        } else {
            IndexLiveness::Released
        };
        probe.events.push(ProducerLeaseEvent {
            consumer,
            point,
            checkpoint,
            working_bytes: WorkingBytes(self.owner.derived.working_bytes()),
            index,
            index_liveness,
        });
    }
    #[cfg(test)]
    pub(in crate::application::file_dag::program) fn take_producer_releases(
        &self,
        state: &RuntimeState,
    ) -> Vec<ProducerLeaseEvent> {
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
    pub(in crate::application::file_dag::program) fn route_observation(
        &self,
        state: &RuntimeState,
        rule: crate::domain::coverage::types::RuleIndex,
        step: ryo_values::domain::plan_parts::types::StepId,
    ) -> Result<crate::domain::operating::types::ExecutionObservation, OperatingError> {
        let origin = self
            .program
            .rules()
            .get(rule.0)
            .and_then(|rule| rule.original_steps.get(step.0 as usize))
            .ok_or(OperatingError::NotRun)?
            .as_ref()
            .map_err(Clone::clone)?;
        let region = self
            .program
            .logical_regions()
            .get(origin.logical.0)
            .ok_or(OperatingError::NotRun)?;
        self.region_observation(state, *region)
    }
    #[cfg(test)]
    pub(in crate::application::file_dag::program) fn region_observation(
        &self,
        state: &RuntimeState,
        region: RegionId,
    ) -> Result<crate::domain::operating::types::ExecutionObservation, OperatingError> {
        self.validate_state(state)?;
        let slots = state.slots.borrow();
        let Some(slot) = slots.get(&region) else {
            self.program
                .regions()
                .get(region.0)
                .ok_or(OperatingError::NotRun)?;
            return Ok(Default::default());
        };
        Ok(crate::domain::operating::types::ExecutionObservation {
            native_positions: slot.native_positions.0,
            registered_calls: slot.registered_calls.0,
            ..slot.observed
        })
    }
    #[cfg(test)]
    pub(in crate::application::file_dag::program) fn retry_checkpoint(
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
                .ok_or(OperatingError::NotRun)?;
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
