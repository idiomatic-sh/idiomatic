//! Retirement of consumed inputs, cascading cancellation and waiter wake-up.
use super::super::{
    AdmissionProgress, ProgramExecution, RetirementWork, RuntimeState, Slot, State,
};
use crate::application::file_dag::program::runtime::types::{Dependencies, ReaderCount};
use ryo_execution_domain::domain::interpreted_program::types::RegionId;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_store::application::file_dag::relations::interface::RelationsPort as _;

impl RetirementWork {
    pub(crate) fn reset(&mut self) {
        self.simulated.clear();
        self.touched.clear();
        self.pending.clear();
    }
}
impl ProgramExecution {
    /// A prepared consumer obligation can finish once. The whole transitive
    /// cancellation is preflighted before any slot, count or lease changes.
    pub(crate) fn release_inputs(
        &self,
        state: &RuntimeState,
        consumer: RegionId,
    ) -> Result<(), OperatingError> {
        let result = self.retire(state, consumer);
        if let Err(error) = &result {
            self.cleanup_failure
                .borrow_mut()
                .get_or_insert(error.clone());
        }
        result
    }
    pub(crate) fn retire(
        &self,
        state: &RuntimeState,
        consumer: RegionId,
    ) -> Result<(), OperatingError> {
        let mut work = state.retirement.borrow_mut();
        work.reset();
        let mut slots = state.slots.borrow_mut();
        let slot = slots.get(&consumer).ok_or_else(|| OperatingError::NotRun)?;
        if slot.dependencies_consumed == Dependencies::Consumed {
            return Err(OperatingError::NotRun);
        }
        work.pending.extend(
            self.program.regions()[consumer.0]
                .dependencies()
                .iter()
                .copied(),
        );
        let planned = (|| {
            while let Some(id) = work.pending.pop() {
                self.program
                    .regions()
                    .get(id.0)
                    .ok_or_else(|| OperatingError::NotRun)?;
                let slot = slots.get(&id);
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
                if current.0 == 0 {
                    return Err(OperatingError::NotRun);
                }
                if current.0 == 1 && slot.is_some_and(|slot| matches!(slot.state, State::Running)) {
                    return Ok(Some(id));
                }
                let remaining = ReaderCount(current.0 - 1);
                work.simulated.insert(id, remaining);
                // An absent root owes no input, whether or not it was ever touched: its edges were never counted.
                if remaining.0 == 0
                    && !slot
                        .is_some_and(|slot| slot.dependencies_consumed == Dependencies::Consumed)
                    && state.absent.borrow().presence(id)
                        == ryo_execution_domain::domain::interpreted_program::presence::types::RegionPresence::Present
                {
                    work.pending
                        .extend(self.program.regions()[id.0].dependencies().iter().copied());
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
                let slot = Slot::touched(state, &mut slots, &self.program, consumer)?;
                if slot.waiting_on.is_none() {
                    slot.waiting_on = Some(blocker);
                    work.waiters.entry(blocker).or_default().push(consumer);
                }
                return Ok(());
            }
            Ok(None) => {}
        }
        let slot = Slot::touched(state, &mut slots, &self.program, consumer)?;
        slot.dependencies_consumed = Dependencies::Consumed;
        slot.waiting_on = None;
        for at in 0..work.touched.len() {
            let id = work.touched[at];
            let Some(slot) = slots.get_mut(&id) else {
                state.dormant.borrow_mut().insert(id, work.simulated[&id]);
                continue;
            };
            slot.remaining = work.simulated[&id];
            if slot.remaining.0 == 0 {
                slot.dependencies_consumed = Dependencies::Consumed;
                slot.waiting_on = None;
                slot.admission = AdmissionProgress::default();
                let old = std::mem::replace(&mut slot.state, State::Retired);
                if matches!(old, State::Published(_)) {
                    self.owner.retire_producer(
                        state.scope,
                        ryo_execution_store::application::file_dag::relations::types::ProducerId(
                            id.0,
                        ),
                    );
                }
                work.retired_states.push(old);
                work.retired_scratch.push(std::mem::take(&mut slot.scratch));
            }
        }
        work.reset();
        let mut retired_states = std::mem::take(&mut work.retired_states);
        let mut retired_scratch = std::mem::take(&mut work.retired_scratch);
        drop(slots);
        drop(work);
        // Lease destructors run outside all scheduler-state borrows.
        retired_states.clear();
        retired_scratch.clear();
        let mut work = state.retirement.borrow_mut();
        work.retired_states = retired_states;
        work.retired_scratch = retired_scratch;
        Ok(())
    }
    pub(crate) fn wake_retirements(&self, state: &RuntimeState, producer: RegionId) {
        // Only a blocker transition wakes its exact consumers. Unchanged
        // running ancestors never cause a full queue or graph rescan.
        let waiters = state.retirement.borrow_mut().waiters.remove(&producer);
        let Some(waiters) = waiters else {
            return;
        };
        for consumer in waiters {
            {
                let mut slots = state.slots.borrow_mut();
                let Some(slot) = slots.get_mut(&consumer) else {
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
    pub(crate) fn cleanup_status(&self) -> Result<(), OperatingError> {
        if let Some(error) = self.cleanup_failure.borrow().as_ref() {
            return Err(error.clone());
        }
        Ok(())
    }
}
