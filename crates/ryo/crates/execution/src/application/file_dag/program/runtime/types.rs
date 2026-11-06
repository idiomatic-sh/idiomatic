//! Independent runtime scratch, inputs, counters and observed results; no issuance or interpreter state.
use crate::application::file_dag::program::admission::AdmittedInput;
#[cfg(test)]
use crate::domain::executable_program::types::RegionId;
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::Value;

pub(in crate::application::file_dag::program) enum InvocationFailure {
    Semantic(OperatingError),
    Delivery(OperatingError),
}
/// No held inputs/publication or dependency accounting at internal instructions.
#[derive(Default)]
pub(in crate::application::file_dag::program) struct KernelScratch {
    pub positions: crate::domain::native_positions::types::PositionWork,
    pub workspace: Option<crate::application::file_dag::workspace::InvocationWorkspace>,
    pub ordered_row: Vec<Value>,
    pub ordered: ryo_operators::inbound::use_cases::operators::interface::OrderedExistenceScratch,
}
pub(in crate::application::file_dag::program) enum KernelInput<'a> {
    Admitted(&'a [AdmittedInput]),
    Continuation(&'a [Value]),
}
#[derive(Default)]
pub(in crate::application::file_dag::program) struct ColumnCounters {
    pub join: crate::domain::operator::contract::types::WorkCounters,
    pub filter: crate::domain::operator::contract::types::WorkCounters,
    pub project: crate::domain::operator::contract::types::WorkCounters,
}
pub(in crate::application::file_dag::program) enum StrategyOutcome {
    Executed,
    Declined,
}
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::program) enum AttemptCheckpoint {
    Dormant,
    Running,
    Published,
    Terminal,
}
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::program) enum DependencyCheckpoint {
    Pending,
    Consumed,
}
/// Prepared consumer obligations a region still owes before it may retire.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::program) struct ReaderCount(
    pub(in crate::application::file_dag::program) usize,
);
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::program) struct RetryCheckpoint {
    pub(in crate::application::file_dag::program) attempt: AttemptCheckpoint,
    pub(in crate::application::file_dag::program) remaining: ReaderCount,
    pub(in crate::application::file_dag::program) dependencies: DependencyCheckpoint,
}
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::program) enum ConsumerReleasePoint {
    Before,
    After,
}
#[cfg(test)]
pub(in crate::application::file_dag::program) struct ProducerLeaseEvent {
    pub consumer: RegionId,
    pub point: ConsumerReleasePoint,
    pub checkpoint: RetryCheckpoint,
    pub working_bytes: WorkingBytes,
    pub index_liveness: IndexLiveness,
    pub index:
        Option<crate::application::file_dag::derived::owner::witness::service::BackingWitness>,
}

pub(in crate::application::file_dag::program) enum NativeJoinEligibility {
    Native,
    Ordinary,
}

/// Bytes of derived working storage held when a producer lease was observed.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::program) struct WorkingBytes(pub usize);

/// Whether the derived index backing a producer was still alive when observed.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::program) enum IndexLiveness {
    Alive,
    Released,
}

/// How many demand actions of a region have been supplied or admitted.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::program) struct ActionCursor(pub usize);

/// Position of an instruction within its region's instruction list.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::program) struct InstructionIndex(pub usize);

/// One step of region admission: not yet performed, or performed once.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::program) enum StageProgress {
    #[default]
    Pending,
    Done,
}

/// Whether a poll only prepares the region for evaluation or evaluates it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::program) enum PollMode {
    PrepareOnly,
    Evaluate,
}

/// Calls a region's kernels made, observed only by tests.
#[cfg(test)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::program) struct CallCount(pub usize);

/// Times a scope derived its reader obligations, observed only by tests.
#[cfg(test)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::program) struct ReaderInitializations(pub usize);

/// Whether a region's dependencies were already released by its consumers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::program) enum Dependencies {
    Pending,
    Consumed,
}
