//! A producer completion wakes exactly the admissions waiting for that producer.
use super::super::{ReadinessAction, ReadinessCounts, ReadinessOutcome, RegionCount, WaitingCount};
use crate::domain::{executable_program::types::RegionId, operating::errors::OperatingError};
use std::collections::VecDeque;

#[derive(Clone, Copy)]
enum State {
    Unseen,
    Queued,
    Polling,
    Waiting(RegionId),
    Ready,
    Terminal,
}
struct Entry {
    state: State,
    waiters: Vec<RegionId>,
}
/// Owns only readiness transitions. The application polls the sole actual interpreter.
pub(crate) struct ReadyQueue {
    entries: Vec<Entry>,
    pending: VecDeque<RegionId>,
    ready: VecDeque<RegionId>,
    cohort: VecDeque<RegionId>,
    blocked: WaitingCount,
    counts: ReadinessCounts,
}
impl ReadyQueue {
    pub(crate) fn new(
        region_count: RegionCount,
        roots: &[RegionId],
    ) -> Result<Self, OperatingError> {
        let mut queue = Self {
            entries: (0..region_count.0)
                .map(|_| Entry {
                    state: State::Unseen,
                    waiters: Vec::new(),
                })
                .collect(),
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
        let entry = self.entries.get_mut(id.0).ok_or(OperatingError::NotRun)?;
        if matches!(entry.state, State::Unseen) {
            entry.state = State::Queued;
            self.pending.push_back(id);
        }
        Ok(())
    }
    /// Finish the current evaluation cohort before admitting newly woken parents.
    /// Otherwise drain all reachable admissions before evaluating any ready head.
    pub(crate) fn next(&mut self) -> Result<Option<ReadinessAction>, OperatingError> {
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
        self.entries[id.0].state = State::Polling;
        Ok(Some(action))
    }
    pub(crate) fn observe(
        &mut self,
        id: RegionId,
        outcome: ReadinessOutcome,
    ) -> Result<(), OperatingError> {
        if !matches!(
            self.entries.get(id.0).map(|entry| entry.state),
            Some(State::Polling)
        ) {
            return Err(OperatingError::NotRun);
        }
        match outcome {
            ReadinessOutcome::Ready => {
                self.entries[id.0].state = State::Ready;
                self.ready.push_back(id);
            }
            ReadinessOutcome::Waiting(dependency) => {
                if dependency == id {
                    return Err(OperatingError::NotRun);
                }
                let dependency_state = self
                    .entries
                    .get(dependency.0)
                    .ok_or(OperatingError::NotRun)?
                    .state;
                self.counts.suspensions.0 += 1;
                if matches!(dependency_state, State::Terminal) {
                    // Canonical admission consumes a completed producer or its
                    // error; another Waiting event cannot make further progress.
                    return Err(OperatingError::NotRun);
                }
                self.entries[id.0].state = State::Waiting(dependency);
                self.entries[dependency.0].waiters.push(id);
                self.blocked.0 += 1;
                self.discover(dependency)?;
            }
            ReadinessOutcome::Terminal => {
                self.entries[id.0].state = State::Terminal;
                for waiter in std::mem::take(&mut self.entries[id.0].waiters) {
                    if !matches!(self.entries[waiter.0].state, State::Waiting(producer) if producer == id)
                    {
                        return Err(OperatingError::NotRun);
                    }
                    self.entries[waiter.0].state = State::Queued;
                    self.blocked.0 -= 1;
                    self.counts.wakeups.0 += 1;
                    self.pending.push_back(waiter);
                }
            }
        }
        Ok(())
    }
    pub(crate) fn counts(&self) -> ReadinessCounts {
        self.counts
    }
}
