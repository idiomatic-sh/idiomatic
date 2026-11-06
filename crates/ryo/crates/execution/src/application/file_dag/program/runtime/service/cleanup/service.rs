//! Retirement of consumed inputs, cascading cancellation and waiter wake-up.
use super::super::{
    AdmissionProgress, ProgramExecution, RetirementWork, RuntimeState, Slot, State,
};
use crate::application::file_dag::program::runtime::types::ConsumerReleasePoint;
use crate::domain::executable_program::types::RegionId;
use crate::domain::operating::errors::OperatingError;

impl RetirementWork {
    pub(in crate::application::file_dag::program::runtime::service) fn reset(&mut self) {
        self.simulated.clear();
        self.touched.clear();
        self.pending.clear();
    }
}
impl ProgramExecution {
    /// A prepared consumer obligation can finish once. The whole transitive
    /// cancellation is preflighted before any slot, count or lease changes.
    pub(in crate::application::file_dag::program::runtime) fn release_inputs(
        &self,
        state: &RuntimeState,
        consumer: RegionId,
    ) -> Result<(), OperatingError> {
        #[cfg(test)]
        self.observe_consumer_release(state, consumer, ConsumerReleasePoint::Before);
        let result = self.retire(state, consumer);
        if let Err(error) = &result {
            self.cleanup_failure
                .borrow_mut()
                .get_or_insert(error.clone());
        }
        #[cfg(test)]
        self.observe_consumer_release(state, consumer, ConsumerReleasePoint::After);
        result
    }
    pub(in crate::application::file_dag::program::runtime) fn retire(
        &self,
        state: &RuntimeState,
        consumer: RegionId,
    ) -> Result<(), OperatingError> {
        let mut work = state.retirement.borrow_mut();
        work.reset();
        let mut slots = state.slots.borrow_mut();
        let slot = slots.get(&consumer.0).ok_or(OperatingError::NotRun)?;
        if slot.dependencies_consumed {
            return Err(OperatingError::NotRun);
        }
        work.pending.extend(
            self.program.regions()[consumer.0]
                .dependencies
                .iter()
                .copied(),
        );
        let planned = (|| {
            while let Some(id) = work.pending.pop() {
                self.program
                    .regions()
                    .get(id.0)
                    .ok_or(OperatingError::NotRun)?;
                let slot = slots.get(&id.0);
                let current = if let Some(current) = work.simulated.get(&id) {
                    *current
                } else {
                    work.touched.push(id);
                    match slot {
                        Some(slot) => slot.remaining,
                        None => match state.dormant.borrow().get(&id).copied() {
                            Some(remaining) => remaining,
                            None => state.initial_remaining(&self.program, id)?,
                        },
                    }
                };
                if current == 0 {
                    return Err(OperatingError::NotRun);
                }
                if current == 1 && slot.is_some_and(|slot| matches!(slot.state, State::Running)) {
                    return Ok(Some(id));
                }
                let remaining = current - 1;
                work.simulated.insert(id, remaining);
                if remaining == 0 && !slot.is_some_and(|slot| slot.dependencies_consumed) {
                    work.pending
                        .extend(self.program.regions()[id.0].dependencies.iter().copied());
                }
            }
            Ok(None)
        })();
        match planned {
            Err(error) => {
                work.reset();
                return Err(error);
            }
            Ok(Some(blocker)) => {
                work.reset();
                if Slot::touched(state, &mut slots, &self.program, consumer)?
                    .waiting_on
                    .is_none()
                {
                    Slot::touched(state, &mut slots, &self.program, consumer)?.waiting_on =
                        Some(blocker);
                    work.waiters.entry(blocker).or_default().push(consumer);
                }
                return Ok(());
            }
            Ok(None) => {}
        }
        Slot::touched(state, &mut slots, &self.program, consumer)?.dependencies_consumed = true;
        Slot::touched(state, &mut slots, &self.program, consumer)?.waiting_on = None;
        for at in 0..work.touched.len() {
            let id = work.touched[at];
            let Some(slot) = slots.get_mut(&id.0) else {
                state.dormant.borrow_mut().insert(id, work.simulated[&id]);
                continue;
            };
            slot.remaining = work.simulated[&id];
            if slot.remaining == 0 {
                slot.dependencies_consumed = true;
                slot.waiting_on = None;
                slot.admission = AdmissionProgress::default();
                let old = std::mem::replace(&mut slot.state, State::Retired);
                if matches!(old, State::Published(_)) {
                    self.owner.release_derived_producer(state.scope, id);
                }
                work.retired.push((old, std::mem::take(&mut slot.scratch)));
            }
        }
        work.reset();
        let mut retired = std::mem::take(&mut work.retired);
        drop(slots);
        drop(work);
        // Lease destructors run outside all scheduler-state borrows.
        retired.clear();
        state.retirement.borrow_mut().retired = retired;
        Ok(())
    }
    pub(in crate::application::file_dag::program::runtime::service) fn wake_retirements(
        &self,
        state: &RuntimeState,
        producer: RegionId,
    ) {
        // Only a blocker transition wakes its exact consumers. Unchanged
        // running ancestors never cause a full queue or graph rescan.
        let waiters = state.retirement.borrow_mut().waiters.remove(&producer);
        let Some(waiters) = waiters else {
            return;
        };
        for consumer in waiters {
            {
                let mut slots = state.slots.borrow_mut();
                let Some(slot) = slots.get_mut(&consumer.0) else {
                    continue;
                };
                if slot.waiting_on != Some(producer) {
                    continue;
                }
                slot.waiting_on = None;
            }
            if let Err(error) = self.release_inputs(state, consumer) {
                self.cleanup_failure.borrow_mut().get_or_insert(error);
            }
        }
    }
    pub(in crate::application::file_dag::program::runtime::service) fn cleanup_status(
        &self,
    ) -> Result<(), OperatingError> {
        if let Some(error) = self.cleanup_failure.borrow().as_ref() {
            return Err(error.clone());
        }
        Ok(())
    }
}
