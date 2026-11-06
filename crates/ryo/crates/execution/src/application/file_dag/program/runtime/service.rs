//! The sole admitted-program interpreter, authority issuance and physical dispatch.
#[cfg(test)]
use super::types::ProducerLeaseEvent;
use super::types::{
    ColumnCounters, InvocationFailure, KernelInput, KernelScratch, StrategyOutcome,
};
use crate::application::file_dag::program::admission::AdmittedInput;
use crate::application::file_dag::program::admission::AvailableInput;
use crate::application::file_dag::program::admission::Inputs;
use crate::application::file_dag::program::admission::Partition;
use crate::application::file_dag::relations::RelationOwner;
use crate::domain::executable_program::types::ExecutableProgram;
use crate::domain::executable_program::types::Instruction;
use crate::domain::executable_program::types::Region;
use crate::domain::executable_program::types::RegionId;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::Emit;
use crate::domain::operating::types::OperatorContext;
use std::cell::RefCell;
use std::sync::Arc;

/// Only this runtime can construct authority to issue a kernel permit.
/// A sibling owning a ProgramExecution reference still cannot skip admission.
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
        Option<crate::application::file_dag::relations::handle::service::PublishedInput>,
        InvocationFailure,
    >;
    fn selected(
        &self,
        shape: &crate::domain::executable_program::selection::SelectionAlternative,
        permit: &KernelPermit<'_>,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
    ) -> Result<
        Option<crate::application::file_dag::relations::handle::service::PublishedInput>,
        InvocationFailure,
    >;
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
#[derive(Default)]
struct AdmissionProgress {
    action: usize,
    availability_action: usize,
    schema_done: bool,
    ordered_done: bool,
    column_done: bool,
    ready: bool,
    preflight_done: bool,
    inputs: Vec<AdmittedInput>,
    acquired: Vec<(
        crate::domain::executable_program::types::InputSource,
        InputStage,
    )>,
    waiting: Option<RegionId>,
    emptiness: Vec<(
        crate::domain::executable_program::types::InputSource,
        crate::domain::operator::contract::types::EmptyRepresentation,
        bool,
    )>,
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
    native_positions: usize,
    #[cfg(test)]
    registered_calls: usize,
    state: State,
    scratch: Vec<KernelScratch>,
    admission: AdmissionProgress,
    fed: Box<[crate::domain::operator::types::RowsFed]>,
    remaining: usize,
    dependencies_consumed: bool,
    waiting_on: Option<RegionId>,
    completion:
        Option<crate::domain::completion::types::completion_policy::value::CompletionPolicy>,
}
/// Reused scope-local planning buffers. No per-row allocation or mutation of
/// reader counts occurs until the entire cancellation cascade validates.
struct RetirementWork {
    simulated: std::collections::HashMap<RegionId, usize>,
    touched: Vec<RegionId>,
    pending: Vec<RegionId>,
    waiters: std::collections::HashMap<RegionId, Vec<RegionId>>,
    retired: Vec<(State, Vec<KernelScratch>)>,
}
pub(crate) struct ProgramExecution {
    program: Arc<ExecutableProgram>,
    owner: std::rc::Rc<crate::application::file_dag::relations::RelationOwner>,
    generation: Arc<()>,
    generation_live:
        Arc<crate::application::file_dag::relations::generation::service::GenerationLease>,
    partition: Partition,
    file_state: RuntimeState,
    project_states: RefCell<
        Vec<(
            crate::application::file_dag::derived::population::types::PopulationId,
            std::rc::Rc<RuntimeState>,
        )>,
    >,
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
    slots: RefCell<std::collections::HashMap<usize, Slot>>,
    dormant: RefCell<std::collections::HashMap<RegionId, usize>>,
    retirement: RefCell<RetirementWork>,
    #[cfg(test)]
    reader_initializations: std::cell::Cell<usize>,
}
struct Continue<'a> {
    invocation: &'a KernelPermit<'a>,
    region: &'a Region,
    at: usize,
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
    prepare_only: bool,
}
#[derive(Clone, Copy)]
enum KernelStrategy {
    Registered,
    Positions,
    Ordered,
    Column,
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
    instruction: usize,
    generation: &'scope Arc<()>,
    partition: &'scope Partition,
    live:
        &'scope Arc<crate::application::file_dag::relations::generation::service::GenerationLease>,
    bindings: BoundInputs<'scope>,
    strategy: KernelStrategy,
    owner: &'scope RelationOwner,
}

/// Pure borrowed semantic authority. It exposes neither worker ownership nor
/// dependency acquisition, and cannot outlive its admitted invocation permit.
pub(in crate::application::file_dag) struct SemanticPermit<'scope> {
    program: &'scope crate::domain::executable_program::types::ExecutableProgram,
    region: RegionId,
    instruction: usize,
    live:
        &'scope Arc<crate::application::file_dag::relations::generation::service::GenerationLease>,
    strategy: KernelStrategy,
}

/// The request and its validated authority cannot be separated or rewritten.
pub(crate) struct PermittedPhysical<'permit, 'meaning, 'scope> {
    permit: &'permit SemanticPermit<'scope>,
    meaning: ryo_operators::inbound::use_cases::operators::interface::OperatorCall<'meaning>,
    authority: crate::domain::executable_program::physical::PhysicalAuthorization,
}

/// The borrowed frame of one poll that the demand stages share; it grants nothing the poll lacked.
pub(in crate::application::file_dag::program::runtime::service) struct Poll<'call, 'ctx> {
    id: RegionId,
    region: &'call Region,
    completion: &'call crate::domain::completion::types::completion_policy::value::CompletionPolicy,
    state: &'call RuntimeState,
    kernels: &'call dyn KernelInvocation,
    prepare_only: bool,
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
