//! Scheduler decisions carry region identities, never evaluation authority.
use crate::domain::executable_program::types::RegionId;
pub mod ready_queue;
/// Distinct physical regions in this compiled program.
pub struct RegionCount(pub usize);
/// Readiness entries a queue holds, one for each region it has reached.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg(test)]
pub(crate) struct ReadinessEntryCount(pub usize);
/// Admissions currently parked on a real producer.
pub(crate) struct WaitingCount(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadinessAction {
    Prepare(RegionId),
    Evaluate(RegionId),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadinessOutcome {
    Waiting(RegionId),
    Ready,
    Terminal,
}
/// Actual canonical readiness interpreter events, independently of row work.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReadinessEvents(pub u64);
/// Work performed by the canonical file/project owner's bound-root preparation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReadinessCounts {
    pub preparations: ReadinessEvents,
    pub evaluations: ReadinessEvents,
    pub suspensions: ReadinessEvents,
    pub wakeups: ReadinessEvents,
}
impl std::ops::AddAssign for ReadinessCounts {
    fn add_assign(&mut self, other: Self) {
        self.preparations.0 += other.preparations.0;
        self.evaluations.0 += other.evaluations.0;
        self.suspensions.0 += other.suspensions.0;
        self.wakeups.0 += other.wakeups.0;
    }
}

/// Actual retained mutable entries, independent of the compiled region count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InvocationStateCount(pub usize);
/// Read-only size of the shared interpreter's live state, not an admission capability.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InvocationStateCounts {
    pub populations: InvocationStateCount,
    pub slots: InvocationStateCount,
    pub instruction_counters: InvocationStateCount,
    pub dormant_obligations: InvocationStateCount,
}
