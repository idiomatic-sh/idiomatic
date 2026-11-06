//! Runtime-owned invocation state and unforgeable capabilities. Fields are visible only within this owner concept.
use crate::application::file_dag::program::admission::AdmittedInput;

use crate::application::file_dag::program::admission::AvailableInput;

use crate::application::file_dag::program::admission::Partition;

use crate::domain::executable_program::types::ExecutableProgram;
use crate::domain::executable_program::types::Instruction;
use crate::domain::executable_program::types::Region;
use crate::domain::executable_program::types::RegionId;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::Emit;
use crate::domain::operating::types::OperatorContext;
use ryo_relations::domain::relations::types::Value;
use std::cell::RefCell;
use std::sync::Arc;

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
/// Only this runtime can construct authority to issue a kernel permit.
/// A sibling owning a ProgramExecution reference still cannot skip admission.
pub(in crate::application::file_dag::program) struct KernelAdmission<'a> {
    pub(super) execution: &'a ProgramExecution,
    pub(super) region: RegionId,
}

/// Only the completed admission interpreter can issue a ready selection head.
pub(in crate::application::file_dag::program) struct ReadySelection<'call, 'scope> {
    pub(super) permit: &'call KernelPermit<'scope>,
}
/// Application-owned preparation/acquisition supplies the borrowed semantic call.
/// All implementations are required; no missing-capability or fallback path.
pub(in crate::application::file_dag::program) trait KernelInvocation {
    fn ready(
        &self,
        _ready: &ReadySelection<'_, '_>,
        _inputs: &[AdmittedInput],
        _context: &mut OperatorContext<'_>,
    ) -> Result<bool, OperatingError> {
        Ok(false)
    }

    fn ordered(
        &self,
        shape: &crate::domain::executable_program::types::OrderedExistence,
        permit: &KernelPermit<'_>,
        input: &AdmittedInput,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
        output: &mut dyn Emit,
    ) -> Result<StrategyOutcome, InvocationFailure>;
    fn positions(
        &self,
        permit: &KernelPermit<'_>,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
    ) -> Result<Option<crate::application::file_dag::relations::PublishedInput>, InvocationFailure>;
    fn selected(
        &self,
        shape: &crate::domain::executable_program::selection::SelectionAlternative,
        permit: &KernelPermit<'_>,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
    ) -> Result<Option<crate::application::file_dag::relations::PublishedInput>, InvocationFailure>;
    fn column(
        &self,
        shape: &crate::domain::executable_program::column::ColumnAlternative,
        permit: &KernelPermit<'_>,
        left: &AdmittedInput,
        right: &AdmittedInput,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
        counts: &ColumnCounters,
        rows: &mut ryo_values::domain::rows::types::Table,
    ) -> Result<StrategyOutcome, InvocationFailure>;
    fn evaluate(
        &self,
        instruction: &Instruction,
        permit: &KernelPermit<'_>,
        input: KernelInput<'_>,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
        output: &mut dyn Emit,
    ) -> Result<(), InvocationFailure>;
}
pub(in crate::application::file_dag::program) enum StrategyOutcome {
    Executed,
    Declined,
}
#[derive(Default)]
pub(super) struct AdmissionProgress {
    pub(super) action: usize,
    pub(super) availability_action: usize,
    pub(super) schema_done: bool,
    pub(super) ordered_done: bool,
    pub(super) column_done: bool,
    pub(super) ready: bool,
    pub(super) preflight_done: bool,
    pub(super) inputs: Vec<AdmittedInput>,
    pub(super) acquired: Vec<(
        crate::domain::executable_program::types::InputSource,
        InputStage,
    )>,
    pub(super) waiting: Option<RegionId>,
    pub(super) emptiness: Vec<(
        crate::domain::executable_program::types::InputSource,
        crate::domain::operator::contract::types::EmptyRepresentation,
        bool,
    )>,
}
pub(super) enum InputStage {
    Available(AvailableInput),
    Admitted(AdmittedInput),
}
pub(super) enum State {
    Dormant,
    Suspended,
    Ready,
    Running,
    Published(AdmittedInput),
    Retired,
    Failed(OperatingError),
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
        Option<std::sync::Weak<crate::application::file_dag::derived::storage::DerivedStorage>>,
}
#[cfg(test)]
pub(super) struct ProducerLeaseProbe {
    pub(super) producer: RegionId,
    pub(super) consumers: Box<[RegionId]>,
    pub(super) events: Vec<ProducerLeaseEvent>,
}
pub(super) struct Slot {
    #[cfg(test)]
    pub(super) observed: crate::domain::operating::types::ExecutionObservation,
    #[cfg(test)]
    pub(super) native_positions: usize,
    #[cfg(test)]
    pub(super) registered_calls: usize,
    pub(super) state: State,
    pub(super) scratch: Vec<KernelScratch>,
    pub(super) admission: AdmissionProgress,
    pub(super) fed: Box<[crate::domain::operator::types::RowsFed]>,
    pub(super) remaining: usize,
    pub(super) dependencies_consumed: bool,
    pub(super) waiting_on: Option<RegionId>,
    pub(super) completion:
        Option<crate::domain::completion::types::completion_policy::value::CompletionPolicy>,
}
/// Reused scope-local planning buffers. No per-row allocation or mutation of
/// reader counts occurs until the entire cancellation cascade validates.
pub(super) struct RetirementWork {
    pub(super) simulated: std::collections::HashMap<RegionId, usize>,
    pub(super) touched: Vec<RegionId>,
    pub(super) pending: Vec<RegionId>,
    pub(super) waiters: std::collections::HashMap<RegionId, Vec<RegionId>>,
    pub(super) retired: Vec<(State, Vec<KernelScratch>)>,
}
pub(crate) struct ProgramExecution {
    pub(super) program: Arc<ExecutableProgram>,
    pub(super) owner: std::rc::Rc<crate::application::file_dag::relations::RelationOwner>,
    pub(super) generation: Arc<()>,
    pub(super) generation_live: Arc<crate::application::file_dag::relations::GenerationLease>,
    pub(super) partition: Partition,
    pub(super) file_state: RuntimeState,
    pub(super) project_states: RefCell<
        Vec<(
            crate::application::file_dag::derived::owner::PopulationId,
            std::rc::Rc<RuntimeState>,
        )>,
    >,
    pub(super) cleanup_failure: RefCell<Option<OperatingError>>,
    pub(super) readiness_counts: RefCell<crate::domain::readiness::types::ReadinessCounts>,
    #[cfg(test)]
    pub(super) producer_lease_probe: RefCell<Option<ProducerLeaseProbe>>,
}
/// Qualified state for the same interpreter; never an independent execution owner.
pub(super) enum RootReaders {
    Declared,
    Selected(
        std::collections::HashMap<
            RegionId,
            crate::domain::executable_program::types::ReaderMultiplicity,
        >,
    ),
}
pub(in crate::application::file_dag::program) struct RuntimeState {
    pub(super) generation: Arc<()>,
    pub(super) scope: crate::application::file_dag::relations::ProducerScope,
    pub(super) roots: RefCell<RootReaders>,
    pub(super) slots: RefCell<std::collections::HashMap<usize, Slot>>,
    pub(super) dormant: RefCell<std::collections::HashMap<RegionId, usize>>,
    pub(super) retirement: RefCell<RetirementWork>,
    #[cfg(test)]
    pub(super) reader_initializations: std::cell::Cell<usize>,
}
pub(super) struct Continue<'a> {
    pub(super) invocation: &'a KernelPermit<'a>,
    pub(super) region: &'a Region,
    pub(super) at: usize,
    pub(super) kernels: &'a dyn KernelInvocation,
    pub(super) scratch: &'a mut [KernelScratch],
    pub(super) output: &'a mut dyn Emit,
}
pub(in crate::application::file_dag::program) enum InvocationOutcome {
    Suspended(RegionId),
    Ready,
    Published(AdmittedInput),
}
pub(super) enum Terminal {
    Retry,
    Suspended,
    Ready,
    Published(AdmittedInput),
    Failed(OperatingError),
}
pub(super) struct Invocation<'a> {
    pub(super) execution: &'a ProgramExecution,
    pub(super) state: &'a RuntimeState,
    pub(super) id: RegionId,
    pub(super) scratch: Option<Vec<KernelScratch>>,
    pub(super) terminal: Terminal,
    pub(super) admission: Option<AdmissionProgress>,
    pub(super) started: Option<std::time::Instant>,
    pub(super) prepare_only: bool,
}
#[derive(Clone, Copy)]
pub(super) enum KernelStrategy {
    Registered,
    Positions,
    Ordered,
    Column,
}
#[derive(Clone, Copy)]
pub(super) enum BoundInputs<'a> {
    Ordinary(&'a [AdmittedInput]),
    Ordered(&'a AdmittedInput),
    Column {
        left: &'a AdmittedInput,
        right: &'a AdmittedInput,
    },
}
pub(crate) struct KernelPermit<'scope> {
    pub(super) program: &'scope crate::domain::executable_program::types::ExecutableProgram,
    pub(super) region: RegionId,
    pub(super) instruction: usize,
    pub(super) generation: &'scope Arc<()>,
    pub(super) partition: &'scope Partition,
    pub(super) live: &'scope Arc<crate::application::file_dag::relations::GenerationLease>,
    pub(super) bindings: BoundInputs<'scope>,
    pub(super) strategy: KernelStrategy,
}

/// Pure borrowed semantic authority. It exposes neither worker ownership nor
/// dependency acquisition, and cannot outlive its admitted invocation permit.
pub(in crate::application::file_dag) struct SemanticPermit<'scope> {
    pub(super) program: &'scope crate::domain::executable_program::types::ExecutableProgram,
    pub(super) region: RegionId,
    pub(super) instruction: usize,
    pub(super) live: &'scope Arc<crate::application::file_dag::relations::GenerationLease>,
    pub(super) strategy: KernelStrategy,
}

/// The request and its validated authority cannot be separated or rewritten.
pub(crate) struct PermittedPhysical<'permit, 'meaning, 'scope> {
    pub(super) permit: &'permit SemanticPermit<'scope>,
    pub(super) meaning:
        ryo_operators::inbound::use_cases::operators::interface::OperatorCall<'meaning>,
    pub(super) authority: crate::domain::executable_program::physical::PhysicalAuthorization,
}
