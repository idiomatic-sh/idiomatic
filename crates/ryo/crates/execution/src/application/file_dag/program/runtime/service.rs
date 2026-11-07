//! The sole admitted-program interpreter, authority issuance and physical dispatch.
use super::types::{
    ActionCursor, ColumnCounters, Dependencies, InstructionIndex, InvocationFailure, KernelInput,
    KernelScratch, PollMode, ReaderCount, StageProgress, StrategyOutcome,
};
#[cfg(test)]
use super::types::{CallCount, ProducerLeaseEvent, ReaderInitializations};
use crate::application::file_dag::program::admission::AdmittedInput;
use crate::application::file_dag::program::admission::AvailableInput;
use crate::application::file_dag::program::admission::Emptiness;
use crate::application::file_dag::program::admission::Inputs;
use crate::application::file_dag::program::admission::Partition;
use crate::application::file_dag::relations::RelationOwner;
use crate::domain::executable_program::types::ExecutableProgram;
use crate::domain::executable_program::types::Instruction;
use crate::domain::executable_program::types::Region;
use crate::domain::executable_program::types::RegionId;
use crate::domain::executable_program::types::RouteKind;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::Emit;
use crate::domain::operating::types::OperatorContext;
use std::cell::RefCell;
use std::sync::Arc;

pub(in crate::application::file_dag::program) struct KernelAdmission<'a> {
    execution: &'a ProgramExecution,
    region: RegionId,
}

/// Only the completed admission interpreter can issue a ready selection head.
pub(in crate::application::file_dag::program) struct ReadySelection<'call, 'scope> {
    permit: &'call KernelPermit<'scope>,
}
/// Application-owned preparation/acquisition supplies the borrowed semantic call.
/// All implementations are required; no missing-capability or fallback path.
pub(in crate::application::file_dag::program) trait KernelInvocation {
    fn ready(
        &self,
        _ready: &ReadySelection<'_, '_>,
        _inputs: &[AdmittedInput],
        _context: &mut OperatorContext<'_>,
    ) -> Result<(), OperatingError>;

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
    ) -> Result<
        crate::application::file_dag::program::runtime::types::Attempt<
            crate::application::file_dag::relations::handle::service::PublishedInput,
        >,
        InvocationFailure,
    >;
    fn selected(
        &self,
        shape: &crate::domain::executable_program::types::SelectionAlternative,
        permit: &KernelPermit<'_>,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
    ) -> Result<
        crate::application::file_dag::program::runtime::types::Attempt<
            crate::application::file_dag::relations::handle::service::PublishedInput,
        >,
        InvocationFailure,
    >;
    fn column(
        &self,
        shape: &crate::domain::executable_program::types::ColumnAlternative,
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
#[derive(Default)]
struct AdmissionProgress {
    action: ActionCursor,
    availability_action: ActionCursor,
    schema_done: StageProgress,
    ordered_done: StageProgress,
    column_done: StageProgress,
    ready: StageProgress,
    preflight_done: StageProgress,
    inputs: Vec<AdmittedInput>,
    acquired: Vec<AcquiredInput>,
    waiting: Option<RegionId>,
    emptiness: Vec<EmptinessAnswer>,
}
struct AcquiredInput {
    source: crate::domain::executable_program::types::InputSource,
    stage: InputStage,
}
struct EmptinessAnswer {
    source: crate::domain::executable_program::types::InputSource,
    representation: crate::domain::operator::contract::types::EmptyRepresentation,
    emptiness: Emptiness,
}
enum InputStage {
    Available(AvailableInput),
    Admitted(AdmittedInput),
}
enum State {
    Dormant,
    Suspended,
    Ready,
    Running,
    Published(AdmittedInput),
    Retired,
    Failed(OperatingError),
}
#[cfg(test)]
struct ProducerLeaseProbe {
    scope: crate::application::file_dag::relations::publication::types::ProducerScope,
    producer: RegionId,
    consumers: Box<[RegionId]>,
    events: Vec<ProducerLeaseEvent>,
}
struct Slot {
    #[cfg(test)]
    observed: crate::domain::operating::types::ExecutionObservation,
    #[cfg(test)]
    native_positions: CallCount,
    #[cfg(test)]
    registered_calls: CallCount,
    state: State,
    scratch: Vec<KernelScratch>,
    admission: AdmissionProgress,
    fed: Box<[crate::domain::operator::types::RowsFed]>,
    remaining: ReaderCount,
    dependencies_consumed: Dependencies,
    waiting_on: Option<RegionId>,
    completion:
        Option<crate::domain::completion::types::completion_policy::value::CompletionPolicy>,
}
/// Reused scope-local planning buffers. No per-row allocation or mutation of
/// reader counts occurs until the entire cancellation cascade validates.
struct RetirementWork {
    simulated: std::collections::HashMap<RegionId, ReaderCount>,
    touched: Vec<RegionId>,
    pending: Vec<RegionId>,
    waiters: std::collections::HashMap<RegionId, Vec<RegionId>>,
    retired_states: Vec<State>,
    retired_scratch: Vec<Vec<KernelScratch>>,
}
struct ProjectState {
    population: crate::application::file_dag::derived::population::types::PopulationId,
    state: std::rc::Rc<RuntimeState>,
}
pub(crate) struct ProgramExecution {
    program: Arc<ExecutableProgram>,
    owner: std::rc::Rc<crate::application::file_dag::relations::RelationOwner>,
    generation: Arc<()>,
    generation_live:
        Arc<crate::application::file_dag::relations::generation::service::GenerationLease>,
    partition: Partition,
    file_state: RuntimeState,
    project_states: RefCell<Vec<ProjectState>>,
    cleanup_failure: RefCell<Option<OperatingError>>,
    readiness_counts: RefCell<crate::domain::readiness::types::ReadinessCounts>,
    #[cfg(test)]
    producer_lease_probe: RefCell<Option<ProducerLeaseProbe>>,
}
/// Qualified state for the same interpreter; never an independent execution owner.
enum RootReaders {
    Declared,
    Selected(
        std::collections::HashMap<
            RegionId,
            crate::domain::executable_program::types::ReaderMultiplicity,
        >,
    ),
}
pub(in crate::application::file_dag::program) struct RuntimeState {
    generation: Arc<()>,
    scope: crate::application::file_dag::relations::publication::types::ProducerScope,
    roots: RefCell<RootReaders>,
    slots: RefCell<std::collections::HashMap<RegionId, Slot>>,
    dormant: RefCell<std::collections::HashMap<RegionId, ReaderCount>>,
    retirement: RefCell<RetirementWork>,
    #[cfg(test)]
    reader_initializations: std::cell::Cell<ReaderInitializations>,
}
struct Continue<'a> {
    invocation: &'a KernelPermit<'a>,
    region: &'a Region,
    at: InstructionIndex,
    kernels: &'a dyn KernelInvocation,
    scratch: &'a mut [KernelScratch],
    output: &'a mut dyn Emit,
}
pub(in crate::application::file_dag::program) enum InvocationOutcome {
    Suspended(RegionId),
    Ready,
    Published(AdmittedInput),
}
enum Terminal {
    Retry,
    Suspended,
    Ready,
    Published(AdmittedInput),
    Failed(OperatingError),
}
struct Invocation<'a> {
    execution: &'a ProgramExecution,
    state: &'a RuntimeState,
    id: RegionId,
    scratch: Option<Vec<KernelScratch>>,
    terminal: Terminal,
    admission: Option<AdmissionProgress>,
    started: Option<std::time::Instant>,
    prepare_only: PollMode,
}
#[derive(Clone, Copy)]
enum BoundInputs<'a> {
    Ordinary(&'a [AdmittedInput]),
    Ordered(&'a AdmittedInput),
    Column {
        left: &'a AdmittedInput,
        right: &'a AdmittedInput,
    },
}
pub(crate) struct KernelPermit<'scope> {
    program: &'scope crate::domain::executable_program::types::ExecutableProgram,
    region: RegionId,
    instruction: InstructionIndex,
    generation: &'scope Arc<()>,
    partition: &'scope Partition,
    live:
        &'scope Arc<crate::application::file_dag::relations::generation::service::GenerationLease>,
    bindings: BoundInputs<'scope>,
    strategy: RouteKind,
    owner: &'scope RelationOwner,
}

/// Pure borrowed semantic authority. It exposes neither worker ownership nor
/// dependency acquisition, and cannot outlive its admitted invocation permit.
pub(in crate::application::file_dag) struct SemanticPermit<'scope> {
    program: &'scope crate::domain::executable_program::types::ExecutableProgram,
    region: RegionId,
    instruction: InstructionIndex,
    live:
        &'scope Arc<crate::application::file_dag::relations::generation::service::GenerationLease>,
    strategy: RouteKind,
}

/// The request and its validated authority cannot be separated or rewritten.
pub(crate) struct PermittedPhysical<'permit, 'meaning, 'scope> {
    permit: &'permit SemanticPermit<'scope>,
    meaning: ryo_operators::domain::contract::types::OperatorCall<'meaning>,
    authority: crate::domain::executable_program::physical::PhysicalAuthorization,
}

/// The borrowed frame of one poll that the demand stages share; it grants nothing the poll lacked.
pub(in crate::application::file_dag::program::runtime::service) struct Poll<'call, 'ctx> {
    id: RegionId,
    region: &'call Region,
    completion: &'call crate::domain::completion::types::completion_policy::value::CompletionPolicy,
    state: &'call RuntimeState,
    kernels: &'call dyn KernelInvocation,
    prepare_only: PollMode,
    inputs: &'call mut dyn Inputs,
    context: &'call mut OperatorContext<'ctx>,
    progress: &'call mut AdmissionProgress,
    frames: &'call mut Option<Vec<KernelScratch>>,
}

mod admission;
mod cleanup;
mod counts;
mod evaluation;
mod invocation;
mod permits;
mod physical;
#[cfg(test)]
mod probes;
mod scheduling;
mod state;
mod strategies;
mod walks;

// Private-state proof lives beneath the issuer, never behind concept-wide record fields.
#[cfg(test)]
mod lifecycle;
