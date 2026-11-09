//! Scheduler decisions carry region identities, never evaluation authority.
use crate::domain::executable_program::types::RegionId;
pub mod ready_queue;
/// Distinct physical regions in this compiled program.
#[derive(Debug, Clone, Copy)]
pub struct RegionCount(pub usize);
/// Readiness entries a queue holds, one for each region it has reached.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg(test)]
pub(crate) struct ReadinessEntryCount(pub usize);
/// Admissions currently parked on a real producer.
#[derive(Debug)]
pub(crate) struct WaitingCount(pub usize);

/// What the admission interpreter does for a region whose inputs are ready.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadinessAction {
    /// Prepares the region.
    Prepare(RegionId),
    /// Evaluates the region.
    Evaluate(RegionId),
}
/// Where a region's admission stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadinessOutcome {
    /// It waits for this region.
    Waiting(RegionId),
    /// It is ready.
    Ready,
    /// It ended.
    Terminal,
}
/// Actual canonical readiness interpreter events, independently of row work.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReadinessEvents(pub u64);
/// Work performed by the canonical file/project owner's bound-root preparation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReadinessCounts {
    /// How many regions were prepared.
    pub preparations: ReadinessEvents,
    /// How many were evaluated.
    pub evaluations: ReadinessEvents,
    /// How many were suspended.
    pub suspensions: ReadinessEvents,
    /// How many were woken.
    pub wakeups: ReadinessEvents,
    /// How many published the empty table because their skip formula failed on the file.
    pub presence_absent: ReadinessEvents,
}
impl std::ops::AddAssign for ReadinessCounts {
    fn add_assign(&mut self, other: Self) {
        self.preparations.0 += other.preparations.0;
        self.evaluations.0 += other.evaluations.0;
        self.suspensions.0 += other.suspensions.0;
        self.wakeups.0 += other.wakeups.0;
        self.presence_absent.0 += other.presence_absent.0;
    }
}

/// Actual retained mutable entries, independent of the compiled region count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InvocationStateCount(pub usize);
/// Read-only size of the shared interpreter's live state, not an admission capability.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InvocationStateCounts {
    /// How many populations are live.
    pub populations: InvocationStateCount,
    /// How many slots are live.
    pub slots: InvocationStateCount,
    /// How many instruction counters are live.
    pub instruction_counters: InvocationStateCount,
    /// How many obligations are dormant.
    pub dormant_obligations: InvocationStateCount,
}
