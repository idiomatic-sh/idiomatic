//! Independent runtime scratch, inputs, counters and observed results; no issuance or interpreter state.
use crate::application::file_dag::program::admission::AdmittedInput;
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
#[cfg(test)]
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
    pub working_bytes: usize,
    pub index_live: bool,
    pub index:
        Option<std::sync::Weak<crate::application::file_dag::derived::owner::acquisition::semantics::service::DerivedStorage>>,
}
