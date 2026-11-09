//! A producer completion wakes exactly the admissions waiting for that producer.
use super::super::{
    ReadinessAction, ReadinessCounts, ReadinessOutcome,
    WaitingCount,
};
use crate::domain::{executable_program::types::{RegionCount, RegionId}, operating::errors::OperatingError};
use std::collections::VecDeque;
use rustc_hash::FxHashMap;

#[derive(Clone, Copy, Debug)]
enum State {
    Unseen,
    Queued,
    /// The action the region was dispatched with; an evaluation inside a preparation poll turns it into `Evaluate`.
    Polling(ReadinessAction),
    Waiting(RegionId),
    Ready,
    Terminal,
}
#[derive(Debug)]
struct Entry {
    state: State,
    waiters: Vec<RegionId>,
}
/// Owns only readiness transitions. The application polls the sole actual interpreter.
#[derive(Debug)]
pub struct ReadyQueue {
    region_count: RegionCount,
    entries: FxHashMap<RegionId, Entry>,
    pending: VecDeque<RegionId>,
    ready: VecDeque<RegionId>,
    cohort: VecDeque<RegionId>,
    blocked: WaitingCount,
    counts: ReadinessCounts,
}
impl ReadyQueue {
    /// A queue over the regions of a program, starting from its roots.
    pub fn new(
        region_count: RegionCount,
        roots: &[RegionId],
    ) -> Result<Self, OperatingError> {
        let mut queue = Self {
            region_count,
            entries: FxHashMap::default(),
            pending: VecDeque::new(),
            ready: VecDeque::new(),
            cohort: VecDeque::new(),
            blocked: WaitingCount(0),
            counts: ReadinessCounts::default(),
        };
        for &root in roots {
            queue.discover(root)?;
        }
        Ok(queue)
    }
    fn discover(&mut self, id: RegionId) -> Result<(), OperatingError> {
        if id.0 >= self.region_count.0 {
            return Err(OperatingError::NotRun);
        }
        let entry = self.entries.entry(id).or_insert_with(|| Entry {
            state: State::Unseen,
            waiters: Vec::new(),
        });
        if matches!(entry.state, State::Unseen) {
            entry.state = State::Queued;
            self.pending.push_back(id);
        }
        Ok(())
    }
    /// Finish the current evaluation cohort before admitting newly woken parents.
    /// Otherwise drain all reachable admissions before evaluating any ready head.
    pub fn next(&mut self) -> Result<Option<ReadinessAction>, OperatingError> {
        let action = if let Some(id) = self.cohort.pop_front() {
            self.counts.evaluations.0 += 1;
            ReadinessAction::Evaluate(id)
        } else if let Some(id) = self.pending.pop_front() {
            self.counts.preparations.0 += 1;
            ReadinessAction::Prepare(id)
        } else if !self.ready.is_empty() {
            std::mem::swap(&mut self.ready, &mut self.cohort);
            return self.next();
        } else if self.blocked.0 != 0 {
            // Every actual dependency is parked, with no producer able to advance.
            return Err(OperatingError::NotRun);
        } else {
            return Ok(None);
        };
        let id = match action {
            ReadinessAction::Prepare(id) | ReadinessAction::Evaluate(id) => id,
        };
        self.entries
            .get_mut(&id)
            .ok_or(OperatingError::NotRun)?
            .state = State::Polling(action);
        Ok(Some(action))
    }
    /// Records that the preparation poll of `id` went on to evaluate it, as a region that joins no shared walk does: the queue
    /// counts that evaluation once, as it counts the evaluations it dispatches, so it stays the only writer of the count.
    pub fn evaluated_in_preparation(&mut self, id: RegionId) -> Result<(), OperatingError> {
        let entry = self.entries.get_mut(&id).ok_or(OperatingError::NotRun)?;
        if !matches!(entry.state, State::Polling(ReadinessAction::Prepare(_))) {
            return Err(OperatingError::NotRun);
        }
        entry.state = State::Polling(ReadinessAction::Evaluate(id));
        self.counts.evaluations.0 += 1;
        Ok(())
    }
    /// Records the outcome of polling a region.
    pub fn observe(
        &mut self,
        id: RegionId,
        outcome: ReadinessOutcome,
    ) -> Result<(), OperatingError> {
        if !matches!(
            self.entries.get(&id).map(|entry| entry.state),
            Some(State::Polling(_))
        ) {
            return Err(OperatingError::NotRun);
        }
        match outcome {
            ReadinessOutcome::Ready => {
                self.entries
                    .get_mut(&id)
                    .ok_or(OperatingError::NotRun)?
                    .state = State::Ready;
                self.ready.push_back(id);
            }
            ReadinessOutcome::Waiting(dependency) => {
                if dependency == id {
                    return Err(OperatingError::NotRun);
                }
                if dependency.0 >= self.region_count.0 {
                    return Err(OperatingError::NotRun);
                }
                let dependency_state = self
                    .entries
                    .get(&dependency)
                    .map_or(State::Unseen, |entry| entry.state);
                self.counts.suspensions.0 += 1;
                if matches!(dependency_state, State::Terminal) {
                    // Canonical admission consumes a completed producer or its
                    // error; another Waiting event cannot make further progress.
                    return Err(OperatingError::NotRun);
                }
                self.entries
                    .get_mut(&id)
                    .ok_or(OperatingError::NotRun)?
                    .state = State::Waiting(dependency);
                self.entries
                    .entry(dependency)
                    .or_insert_with(|| Entry {
                        state: State::Unseen,
                        waiters: Vec::new(),
                    })
                    .waiters
                    .push(id);
                self.blocked.0 += 1;
                self.discover(dependency)?;
            }
            ReadinessOutcome::Terminal => {
                self.entries
                    .get_mut(&id)
                    .ok_or(OperatingError::NotRun)?
                    .state = State::Terminal;
                let waiters = std::mem::take(
                    &mut self
                        .entries
                        .get_mut(&id)
                        .ok_or(OperatingError::NotRun)?
                        .waiters,
                );
                for waiter in waiters {
                    if !matches!(self.entries.get_mut(&waiter).ok_or(OperatingError::NotRun)?.state, State::Waiting(producer) if producer == id)
                    {
                        return Err(OperatingError::NotRun);
                    }
                    self.entries
                        .get_mut(&waiter)
                        .ok_or(OperatingError::NotRun)?
                        .state = State::Queued;
                    self.blocked.0 -= 1;
                    self.counts.wakeups.0 += 1;
                    self.pending.push_back(waiter);
                }
            }
        }
        Ok(())
    }
    #[cfg(test)]
    pub(super) fn reached_entries(&self) -> crate::domain::readiness::types::ReadinessEntryCount {
        crate::domain::readiness::types::ReadinessEntryCount(self.entries.len())
    }
    /// The work the queue performed.
    pub fn counts(&self) -> ReadinessCounts {
        self.counts
    }
}
