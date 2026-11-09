//! The sole admitted-program interpreter, authority issuance and physical dispatch.
#[cfg(test)]
use super::types::CallCount;
use super::types::{
    ActionCursor, Dependencies, InstructionIndex, InvocationFailure, KernelInput, KernelScratch,
    PollMode, ReaderCount, StageProgress,
};
#[cfg(test)]
use super::types::{ProducerLeaseEvent, ReaderInitializations};
use crate::application::file_dag::program::admission::AdmittedInput;
use crate::application::file_dag::program::admission::AvailableInput;
use crate::application::file_dag::program::admission::Emptiness;
use crate::application::file_dag::program::admission::Inputs;
use crate::application::file_dag::program::admission::Partition;
use ryo_execution_domain::domain::executable_program::types::RegionId;
use ryo_execution_domain::domain::executable_program::types::RouteKind;
use ryo_execution_domain::domain::executable_program::types::executable_program::value::ExecutableProgram;
use ryo_execution_domain::domain::executable_program::types::instruction::value::Instruction;
use ryo_execution_domain::domain::executable_program::types::region::value::Region;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::types::Emit;
use ryo_execution_domain::domain::operator::types::OperatorContext;
use ryo_execution_store::application::file_dag::relations::RelationOwner;
use std::cell::RefCell;
use std::sync::Arc;

pub(crate) struct KernelAdmission<'a> {
    execution: &'a ProgramExecution,
    region: RegionId,
}

/// Only the completed admission interpreter can issue a ready selection head.
pub(crate) struct ReadySelection<'call, 'scope> {
    permit: &'call KernelPermit<'scope>,
}
/// Application-owned preparation/acquisition supplies the borrowed semantic call.
/// All implementations are required; no missing-capability or fallback path.
pub(crate) trait KernelInvocation {
    /// Whether the ready region joined a shared walk: a joined region is evaluated with its cohort,
    /// after every reachable admission; any other region is evaluated in the poll that found it ready.
    fn ready(
        &self,
        _ready: &ReadySelection<'_, '_>,
        _inputs: &[AdmittedInput],
        _context: &mut OperatorContext<'_>,
    ) -> Result<crate::application::file_dag::program::runtime::types::Submission, OperatingError>;

    fn ordered(
        &self,
        shape: &ryo_execution_domain::domain::executable_program::types::OrderedExistence,
        permit: &KernelPermit<'_>,
        input: &AdmittedInput,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
        output: &mut dyn Emit,
    ) -> Result<(), InvocationFailure>;
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
    ready: StageProgress,
    preflight_done: StageProgress,
    inputs: Vec<AdmittedInput>,
    acquired: Vec<AcquiredInput>,
    waiting: Option<RegionId>,
    emptiness: Vec<EmptinessAnswer>,
}
struct AcquiredInput {
    source: ryo_execution_domain::domain::executable_program::types::InputSource,
    stage: InputStage,
}
struct EmptinessAnswer {
    source: ryo_execution_domain::domain::executable_program::types::InputSource,
    representation: ryo_execution_domain::domain::operator::contract::types::EmptyRepresentation,
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
    scope: ryo_execution_store::application::file_dag::relations::publication::types::ProducerScope,
    producer: RegionId,
    consumers: Box<[RegionId]>,
    events: Vec<ProducerLeaseEvent>,
}
struct Slot {
    #[cfg(test)]
    observed: ryo_execution_domain::domain::operating::fixtures::ExecutionObservation,
    #[cfg(test)]
    registered_calls: CallCount,
    state: State,
    scratch: Vec<KernelScratch>,
    admission: AdmissionProgress,
    fed: Box<[ryo_execution_domain::domain::operator::types::RowsFed]>,
    remaining: ReaderCount,
    dependencies_consumed: Dependencies,
    waiting_on: Option<RegionId>,
    completion: Option<
        ryo_execution_domain::domain::completion::types::completion_policy::value::CompletionPolicy,
    >,
}
/// Reused scope-local planning buffers. No per-row allocation or mutation of
/// reader counts occurs until the entire cancellation cascade validates.
struct RetirementWork {
    simulated: rustc_hash::FxHashMap<RegionId, ReaderCount>,
    touched: Vec<RegionId>,
    pending: Vec<RegionId>,
    waiters: rustc_hash::FxHashMap<RegionId, Vec<RegionId>>,
    retired_states: Vec<State>,
    retired_scratch: Vec<Vec<KernelScratch>>,
}
struct ProjectState {
    population:
        ryo_execution_store::application::file_dag::derived::population::types::PopulationId,
    state: std::rc::Rc<RuntimeState>,
}
pub(crate) struct ProgramExecution {
    program: Arc<ExecutableProgram>,
    owner: std::rc::Rc<RelationOwner>,
    /// The owner's generation when this execution opened; its liveness is the execution's.
    generation: Arc<
        ryo_execution_store::application::file_dag::relations::generation::service::GenerationLease,
    >,
    partition: Partition,
    file_state: RuntimeState,
    project_states: RefCell<Vec<ProjectState>>,
    cleanup_failure: RefCell<Option<OperatingError>>,
    readiness_counts: RefCell<ryo_execution_domain::domain::readiness::types::ReadinessCounts>,
    #[cfg(test)]
    producer_lease_probe: RefCell<Option<ProducerLeaseProbe>>,
}
/// Qualified state for the same interpreter; never an independent execution owner.
enum RootReaders {
    Declared,
    Selected(Arc<ryo_execution_domain::domain::executable_program::types::RemainingTable>),
}
pub(crate) struct RuntimeState {
    generation: Arc<
        ryo_execution_store::application::file_dag::relations::generation::service::GenerationLease,
    >,
    scope: ryo_execution_store::application::file_dag::relations::publication::types::ProducerScope,
    roots: RefCell<RootReaders>,
    slots: RefCell<rustc_hash::FxHashMap<RegionId, Slot>>,
    dormant: RefCell<rustc_hash::FxHashMap<RegionId, ReaderCount>>,
    retirement: RefCell<RetirementWork>,
    /// The file roots this file found absent: each publishes the empty table and owes no input.
    absent: RefCell<ryo_execution_domain::domain::executable_program::file_demand::types::AbsentRoots>,
    /// The other regions the file's admitted roots reach whose skip formula fails on the file.
    interior: RefCell<ryo_execution_domain::domain::executable_program::file_demand::types::AbsentRegions>,
    /// The sources the file's presence read met refused, each the file's refusal for that source.
    refused: RefCell<Vec<super::types::SourceRefusal>>,
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
pub(crate) enum InvocationOutcome {
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
}
pub(crate) struct KernelPermit<'scope> {
    program: &'scope ExecutableProgram,
    region: RegionId,
    instruction: InstructionIndex,
    generation: &'scope Arc<
        ryo_execution_store::application::file_dag::relations::generation::service::GenerationLease,
    >,
    partition: &'scope Partition,
    bindings: BoundInputs<'scope>,
    strategy: RouteKind,
    owner: &'scope RelationOwner,
}

/// Pure borrowed semantic authority. It exposes neither worker ownership nor
/// dependency acquisition, and cannot outlive its admitted invocation permit.
pub(crate) struct SemanticPermit<'scope> {
    program: &'scope ExecutableProgram,
    region: RegionId,
    instruction: InstructionIndex,
    live: &'scope Arc<
        ryo_execution_store::application::file_dag::relations::generation::service::GenerationLease,
    >,
    strategy: RouteKind,
}

/// The request and its validated authority cannot be separated or rewritten.
pub(crate) struct PermittedPhysical<'permit, 'meaning, 'scope> {
    permit: &'permit SemanticPermit<'scope>,
    meaning: ryo_operators::inbound::use_cases::operators::types::OperatorCall<'meaning>,
    authority: ryo_execution_domain::domain::executable_program::physical::PhysicalAuthorization,
}

/// The borrowed frame of one poll that the demand stages share; it grants nothing the poll lacked.
pub(crate) struct Poll<'call, 'ctx> {
    id: RegionId,
    state: &'call RuntimeState,
    region: &'call Region,
    completion: &'call ryo_execution_domain::domain::completion::types::completion_policy::value::CompletionPolicy,
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
