//! Independent runtime scratch, inputs, counters and observed results; no issuance or interpreter state.
use crate::application::file_dag::program::admission::AdmittedInput;
#[cfg(test)]
use ryo_execution_domain::domain::executable_program::types::RegionId;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::Value;

pub(crate) enum InvocationFailure {
    Semantic(OperatingError),
}
/// No held inputs/publication or dependency accounting at internal instructions.
#[derive(Default)]
pub(crate) struct KernelScratch {
    pub workspace: Option<ryo_execution_store::application::file_dag::workspace::InvocationWorkspace>,
    pub ordered_row: Vec<Value>,
    pub ordered: ryo_operators::domain::ordered_existence::types::OrderedExistenceScratch,
}
pub(crate) enum KernelInput<'a> {
    Admitted(&'a [AdmittedInput]),
    Continuation(&'a [Value]),
}
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AttemptCheckpoint {
    Dormant,
    Running,
    Published,
    Terminal,
}
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DependencyCheckpoint {
    Pending,
    Consumed,
}
/// Prepared consumer obligations a region still owes before it may retire.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ReaderCount(
    pub(crate) usize,
);
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RetryCheckpoint {
    pub(crate) attempt: AttemptCheckpoint,
    pub(crate) remaining: ReaderCount,
    pub(crate) dependencies: DependencyCheckpoint,
}
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConsumerReleasePoint {
    Before,
    After,
}
#[cfg(test)]
pub(crate) struct ProducerLeaseEvent {
    pub consumer: RegionId,
    pub point: ConsumerReleasePoint,
    pub checkpoint: RetryCheckpoint,
    pub working_bytes: WorkingBytes,
    pub index_liveness: IndexLiveness,
    pub index:
        Option<ryo_execution_store::application::file_dag::derived::owner::witness::service::BackingWitness>,
}

/// Bytes of derived working storage held when a producer lease was observed.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WorkingBytes(pub usize);

/// Whether the derived index backing a producer was still alive when observed.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IndexLiveness {
    Alive,
    Released,
}

/// How many demand actions of a region have been supplied or admitted.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(crate) struct ActionCursor(pub usize);

/// Position of an instruction within its region's instruction list.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(crate) struct InstructionIndex(pub usize);

/// One step of region admission: not yet performed, or performed once.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(crate) enum StageProgress {
    #[default]
    Pending,
    Done,
}

/// Whether the admitted source joined the shared walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Submission {
    Submitted,
    Declined,
}

/// Whether a poll only prepares the region for evaluation or evaluates it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PollMode {
    PrepareOnly,
    Evaluate,
}

/// Calls a region's kernels made, observed only by tests.
#[cfg(test)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(crate) struct CallCount(pub usize);

/// Times a scope derived its reader obligations, observed only by tests.
#[cfg(test)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(crate) struct ReaderInitializations(pub usize);

/// Whether a scope holds an entry for a region, observed only by tests.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SlotHolding {
    Held,
    Missing,
}
/// Whether a scope holds a slot and a dormant reader count for a region, observed only by tests.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Instantiation {
    pub(crate) slot: SlotHolding,
    pub(crate) dormant: SlotHolding,
}
/// Whether a region's dependencies were already released by its consumers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Dependencies {
    Pending,
    Consumed,
}
