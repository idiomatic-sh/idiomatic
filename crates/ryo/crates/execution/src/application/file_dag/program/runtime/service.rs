//! The sole admitted-program interpreter, authority issuance and physical dispatch.
use super::types::*;
use crate::application::file_dag::program::admission::AdmittedInput;
use crate::application::file_dag::program::admission::AdmittedWalkSource;

use crate::application::file_dag::program::admission::AvailableInput;
use crate::application::file_dag::program::admission::FileGeneration;
use crate::application::file_dag::program::admission::Inputs;
use crate::application::file_dag::program::admission::Partition;
use crate::application::file_dag::program::admission::ProducerRoot;
use crate::application::file_dag::relations::RelationOwner;
use crate::domain::executable_program::types::DemandAction;
use crate::domain::executable_program::types::ExecutableProgram;
use crate::domain::executable_program::types::InputId;
use crate::domain::executable_program::types::InputSource;
use crate::domain::executable_program::types::Instruction;
use crate::domain::executable_program::types::Region;
use crate::domain::executable_program::types::RegionId;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::Emit;
use crate::domain::operating::types::OperatorContext;
use ryo_relations::domain::relations::types::Value;
use std::cell::RefCell;
use std::rc::Rc;
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
    generation_live: Arc<crate::application::file_dag::relations::GenerationLease>,
    partition: Partition,
    file_state: RuntimeState,
    project_states: RefCell<
        Vec<(
            crate::application::file_dag::derived::owner::PopulationId,
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
    scope: crate::application::file_dag::relations::ProducerScope,
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
    live: &'scope Arc<crate::application::file_dag::relations::GenerationLease>,
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
    live: &'scope Arc<crate::application::file_dag::relations::GenerationLease>,
    strategy: KernelStrategy,
}

/// The request and its validated authority cannot be separated or rewritten.
pub(crate) struct PermittedPhysical<'permit, 'meaning, 'scope> {
    permit: &'permit SemanticPermit<'scope>,
    meaning: ryo_operators::inbound::use_cases::operators::interface::OperatorCall<'meaning>,
    authority: crate::domain::executable_program::physical::PhysicalAuthorization,
}

impl<'a> KernelAdmission<'a> {
    pub(in crate::application::file_dag::program) fn execution(&self) -> &'a ProgramExecution {
        self.execution
    }
    pub(in crate::application::file_dag::program) fn region(&self) -> RegionId {
        self.region
    }
}
impl ReadySelection<'_, '_> {
    pub(in crate::application::file_dag::program) fn permit(&self) -> &KernelPermit<'_> {
        self.permit
    }
}
fn schema_first(region: &Region) -> bool {
    region.instructions.first().is_some_and(|instruction| {
        instruction.contract.semantics.preflight
            == crate::domain::operator::contract::types::PreflightOrder::SchemaFirst
    })
}
impl AdmissionProgress {
    fn is_empty(
        &mut self,
        inputs: &dyn Inputs,
        source: crate::domain::executable_program::types::InputSource,
        input: &AdmittedInput,
        representation: crate::domain::operator::contract::types::EmptyRepresentation,
    ) -> Result<bool, OperatingError> {
        if let Some((_, _, empty)) = self
            .emptiness
            .iter()
            .find(|(key, kind, _)| *key == source && *kind == representation)
        {
            return Ok(*empty);
        }
        let empty = inputs.is_empty(input, representation)?;
        self.emptiness.push((source, representation, empty));
        Ok(empty)
    }

    fn check_schema(&mut self, region: &Region) -> Result<(), OperatingError> {
        if !self.schema_done {
            for instruction in region.instructions.iter().rev() {
                if instruction.contract.semantics.preflight
                    == crate::domain::operator::contract::types::PreflightOrder::SchemaFirst
                {
                    instruction
                        .contract
                        .output_kinds
                        .as_ref()
                        .map_err(Clone::clone)?;
                }
            }
            self.schema_done = true;
        }
        Ok(())
    }
    fn supply(
        &mut self,
        inputs: &mut dyn Inputs,
        source: crate::domain::executable_program::types::InputSource,
        partition: &Partition,
        context: &mut OperatorContext<'_>,
    ) -> Result<(), OperatingError> {
        if self.acquired.iter().any(|(key, _)| *key == source) {
            return Ok(());
        }
        if let Some(dependency) = inputs.dependency(source).inspect_err(|error| {
            tracing::trace!(target: "ryo::diagnostic::execution", event = "input_admission_refused", input_phase = "dependency", action = self.action, ?source, ?error);
        })? {
            self.waiting = Some(dependency);
            return Err(OperatingError::NotRun);
        }
        let available = inputs.supply(source, partition, context).inspect_err(|error| {
            tracing::trace!(target: "ryo::diagnostic::execution", event = "input_admission_refused", input_phase = "availability", action = self.action, ?source, ?error);
        })?;
        self.acquired
            .push((source, InputStage::Available(available)));
        Ok(())
    }
    fn admit(
        &mut self,
        inputs: &mut dyn Inputs,
        source: crate::domain::executable_program::types::InputSource,
        partition: &Partition,
        context: &mut OperatorContext<'_>,
    ) -> Result<AdmittedInput, OperatingError> {
        self.supply(inputs, source, partition, context)?;
        let (_, stage) = self
            .acquired
            .iter_mut()
            .find(|(key, _)| *key == source)
            .ok_or(OperatingError::NotRun)?;
        match stage {
            InputStage::Admitted(input) => Ok(input.clone()),
            InputStage::Available(available) => {
                let admitted = inputs.admit(available, partition, context).inspect_err(|error| {
                    tracing::trace!(target: "ryo::diagnostic::execution", event = "input_admission_refused", input_phase = "physical", action = self.action, ?source, ?error);
                })?;
                *stage = InputStage::Admitted(admitted.clone());
                Ok(admitted)
            }
        }
    }
}
impl RetirementWork {
    fn reset(&mut self) {
        self.simulated.clear();
        self.touched.clear();
        self.pending.clear();
    }
}
impl Slot {
    pub(super) fn touched<'a>(
        state: &RuntimeState,
        slots: &'a mut std::collections::HashMap<usize, Slot>,
        program: &ExecutableProgram,
        id: RegionId,
    ) -> Result<&'a mut Slot, OperatingError> {
        let region = program.regions().get(id.0).ok_or(OperatingError::NotRun)?;
        let vacant = match slots.entry(id.0) {
            std::collections::hash_map::Entry::Occupied(entry) => return Ok(entry.into_mut()),
            std::collections::hash_map::Entry::Vacant(entry) => entry,
        };
        let settled = state.dormant.borrow_mut().remove(&id);
        let remaining = match settled {
            Some(remaining) => remaining,
            None => state.initial_remaining(program, id)?,
        };
        let cancelled = remaining == 0;
        // Compiled obligations include roots in other project populations. They
        // remain a conservative upper bound until phase finish; no bank may
        // retire another population's publication or claim exact last use.
        Ok(vacant.insert(Self {
            #[cfg(test)]
            observed: Default::default(),
            #[cfg(test)]
            native_positions: 0,
            #[cfg(test)]
            registered_calls: 0,
            state: if cancelled {
                State::Retired
            } else {
                State::Dormant
            },
            scratch: Vec::new(),
            admission: Default::default(),
            fed: vec![
                crate::domain::operator::types::RowsFed::default();
                region.instructions.len()
            ]
            .into_boxed_slice(),
            remaining,
            dependencies_consumed: cancelled,
            waiting_on: None,
            completion: None,
        }))
    }
}
impl RuntimeState {
    fn new(
        scope: crate::application::file_dag::relations::ProducerScope,
        generation: Arc<()>,
    ) -> Self {
        Self {
            generation,
            scope,
            roots: RefCell::new(RootReaders::Declared),
            #[cfg(test)]
            reader_initializations: std::cell::Cell::new(0),
            slots: RefCell::new(std::collections::HashMap::new()),
            dormant: RefCell::new(std::collections::HashMap::new()),
            retirement: RefCell::new(RetirementWork {
                simulated: std::collections::HashMap::new(),
                touched: Vec::new(),
                pending: Vec::new(),
                waiters: std::collections::HashMap::new(),
                retired: Vec::new(),
            }),
        }
    }
    fn initial_remaining(
        &self,
        program: &ExecutableProgram,
        id: RegionId,
    ) -> Result<usize, OperatingError> {
        #[cfg(test)]
        self.reader_initializations
            .set(self.reader_initializations.get() + 1);
        match &*self.roots.borrow() {
            RootReaders::Declared => program
                .regions()
                .get(id.0)
                .map(|region| region.consumers)
                .ok_or(OperatingError::NotRun),
            RootReaders::Selected(selected) => {
                crate::domain::executable_program::file_demand::service::remaining(
                    program.file_readers(),
                    selected,
                    id,
                )
                .map(|readers| readers.0)
            }
        }
    }
    pub(in crate::application::file_dag::program) fn population_scope(
        &self,
    ) -> crate::application::file_dag::relations::ProducerScope {
        self.scope
    }
    pub(in crate::application::file_dag::program) fn bind(
        &self,
        owner: &crate::application::file_dag::relations::RelationOwner,
        root: crate::application::file_dag::program::admission::ProducerRoot,
        publication: &crate::application::file_dag::relations::PublishedInput,
    ) -> Result<(), OperatingError> {
        if let crate::application::file_dag::program::admission::ProducerRoot::Region(id) = root {
            owner.bind_producer(self.scope, id, publication.as_ref())?;
        }
        Ok(())
    }
}
/// Constructors are private to the outer execution owner; no primitive can open
/// another scope or choose a scheduler.
impl ProgramExecution {
    pub(in crate::application::file_dag::program) fn open(
        program: Arc<ExecutableProgram>,
        partition: Partition,
        owner: std::rc::Rc<crate::application::file_dag::relations::RelationOwner>,
    ) -> Self {
        let generation = owner.generation_guard();
        let generation_live = owner.generation_liveness();
        let file_state = RuntimeState::new(
            crate::application::file_dag::relations::ProducerScope::File,
            Arc::clone(&generation),
        );
        Self {
            program,
            owner,
            generation,
            generation_live,
            partition,
            file_state,
            project_states: RefCell::new(Vec::new()),
            cleanup_failure: RefCell::new(None),
            readiness_counts: RefCell::new(Default::default()),
            #[cfg(test)]
            producer_lease_probe: RefCell::new(None),
        }
    }
    /// Bind the immutable declared file intents before any region is touched.
    pub(in crate::application::file_dag::program) fn activate_file_roots(
        &self,
        readers: &[RegionId],
    ) -> Result<(), OperatingError> {
        if !matches!(self.partition, Partition::File(_))
            || !self.file_state.slots.borrow().is_empty()
            || !self.file_state.dormant.borrow().is_empty()
            || !matches!(*self.file_state.roots.borrow(), RootReaders::Declared)
        {
            return Err(OperatingError::NotRun);
        }
        let mut selected = std::collections::HashMap::new();
        for &root in readers {
            if !self.program.file_readers().consumers.contains_key(&root) {
                return Err(OperatingError::NotRun);
            }
            selected
                .entry(root)
                .or_insert(crate::domain::executable_program::types::ReaderMultiplicity(0))
                .0 += 1;
        }
        *self.file_state.roots.borrow_mut() = RootReaders::Selected(selected);
        Ok(())
    }
    fn validate_state(&self, state: &RuntimeState) -> Result<(), OperatingError> {
        if Arc::ptr_eq(&self.generation, &state.generation) && self.generation_live.is_live() {
            Ok(())
        } else {
            Err(OperatingError::NotRead)
        }
    }
    fn record_column(
        &self,
        state: &RuntimeState,
        shape: &crate::domain::executable_program::column::ColumnAlternative,
        counts: &ColumnCounters,
    ) -> Result<(), OperatingError> {
        let mut slots = state.slots.borrow_mut();
        for (id, values) in [
            (shape.join(), counts.join.snapshot()),
            (shape.filter(), counts.filter.snapshot()),
            (shape.target(), counts.project.snapshot()),
        ] {
            let slot = Slot::touched(state, &mut slots, &self.program, id)?;
            let fed = slot.fed.first_mut().ok_or(OperatingError::NotRun)?;
            for (total, found) in fed.0.iter_mut().zip(values) {
                *total = total.saturating_add(found);
            }
        }
        Ok(())
    }
    /// Observe actual release boundaries without retaining a producer or a backing lease.
    #[cfg(test)]
    pub(in crate::application::file_dag::program) fn observe_producer_releases(
        &self,
        state: &RuntimeState,
        producer: RegionId,
        consumers: &[RegionId],
    ) -> Result<(), OperatingError> {
        if producer.0 >= self.program.regions().len()
            || consumers
                .iter()
                .any(|id| id.0 >= self.program.regions().len())
            || self.producer_lease_probe.borrow().is_some()
        {
            return Err(OperatingError::NotRun);
        }
        *self.producer_lease_probe.borrow_mut() = Some(ProducerLeaseProbe {
            producer,
            consumers: consumers.into(),
            events: vec![],
        });
        Ok(())
    }
    #[cfg(test)]
    fn observe_consumer_release(
        &self,
        state: &RuntimeState,
        consumer: RegionId,
        point: ConsumerReleasePoint,
    ) {
        let mut probe = self.producer_lease_probe.borrow_mut();
        let Some(probe) = probe
            .as_mut()
            .filter(|probe| probe.consumers.contains(&consumer))
        else {
            return;
        };
        let checkpoint = self
            .retry_checkpoint(state, probe.producer)
            .expect("observed actual producer");
        let index = self
            .owner
            .derived
            .view_weak(ryo_derived::domain::contract::types::DerivedKind::Equality);
        let index_live = index
            .as_ref()
            .is_some_and(|index| index.upgrade().is_some());
        probe.events.push(ProducerLeaseEvent {
            consumer,
            point,
            checkpoint,
            working_bytes: self.owner.derived.working_bytes(),
            index,
            index_live,
        });
    }
    #[cfg(test)]
    pub(in crate::application::file_dag::program) fn take_producer_releases(
        &self,
        state: &RuntimeState,
    ) -> Vec<ProducerLeaseEvent> {
        self.producer_lease_probe
            .borrow_mut()
            .take()
            .expect("installed read-only lifecycle observer")
            .events
    }
    /// Observes completed dispatches even after the real retirement drops invocation scratch.
    #[cfg(test)]
    pub(in crate::application::file_dag::program) fn route_observation(
        &self,
        state: &RuntimeState,
        rule: crate::domain::coverage::types::RuleIndex,
        step: ryo_values::domain::plan_parts::types::StepId,
    ) -> Result<crate::domain::operating::types::ExecutionObservation, OperatingError> {
        let origin = self
            .program
            .rules()
            .get(rule.0)
            .and_then(|rule| rule.original_steps.get(step.0 as usize))
            .ok_or(OperatingError::NotRun)?
            .as_ref()
            .map_err(Clone::clone)?;
        let region = self
            .program
            .logical_regions()
            .get(origin.logical.0)
            .ok_or(OperatingError::NotRun)?;
        self.region_observation(state, *region)
    }
    #[cfg(test)]
    pub(in crate::application::file_dag::program) fn region_observation(
        &self,
        state: &RuntimeState,
        region: RegionId,
    ) -> Result<crate::domain::operating::types::ExecutionObservation, OperatingError> {
        self.validate_state(state)?;
        let slots = state.slots.borrow();
        let Some(slot) = slots.get(&region.0) else {
            self.program
                .regions()
                .get(region.0)
                .ok_or(OperatingError::NotRun)?;
            return Ok(Default::default());
        };
        Ok(crate::domain::operating::types::ExecutionObservation {
            native_positions: slot.native_positions,
            registered_calls: slot.registered_calls,
            ..slot.observed
        })
    }
    #[cfg(test)]
    pub(in crate::application::file_dag::program) fn retry_checkpoint(
        &self,
        state: &RuntimeState,
        id: RegionId,
    ) -> Result<RetryCheckpoint, OperatingError> {
        self.validate_state(state)?;
        let slots = state.slots.borrow();
        let Some(slot) = slots.get(&id.0) else {
            self.program
                .regions()
                .get(id.0)
                .ok_or(OperatingError::NotRun)?;
            let remaining = match state.dormant.borrow().get(&id).copied() {
                Some(remaining) => remaining,
                None => state.initial_remaining(&self.program, id)?,
            };
            return Ok(RetryCheckpoint {
                attempt: if state.dormant.borrow().get(&id) == Some(&0) {
                    AttemptCheckpoint::Terminal
                } else {
                    AttemptCheckpoint::Dormant
                },
                remaining: ReaderCount(remaining),
                dependencies: if state.dormant.borrow().get(&id) == Some(&0) {
                    DependencyCheckpoint::Consumed
                } else {
                    DependencyCheckpoint::Pending
                },
            });
        };
        Ok(RetryCheckpoint {
            attempt: match slot.state {
                State::Dormant => AttemptCheckpoint::Dormant,
                State::Running | State::Suspended | State::Ready => AttemptCheckpoint::Running,
                State::Published(_) => AttemptCheckpoint::Published,
                State::Retired | State::Failed(_) => AttemptCheckpoint::Terminal,
            },
            remaining: ReaderCount(slot.remaining),
            dependencies: if slot.dependencies_consumed {
                DependencyCheckpoint::Consumed
            } else {
                DependencyCheckpoint::Pending
            },
        })
    }
    /// Consume the generation once no invocation can remain active. Dropping
    /// its slots releases only engine-held storage; transferred leases own theirs.
    pub(in crate::application::file_dag::program) fn state_counts(
        &self,
    ) -> crate::domain::readiness::types::InvocationStateCounts {
        use crate::domain::readiness::types::{InvocationStateCount, InvocationStateCounts};
        let states = self.project_states.borrow();
        let mut found = InvocationStateCounts {
            populations: InvocationStateCount(states.len()),
            ..Default::default()
        };
        for state in
            std::iter::once(&self.file_state).chain(states.iter().map(|(_, state)| state.as_ref()))
        {
            let slots = state.slots.borrow();
            found.slots.0 += slots.len();
            found.instruction_counters.0 +=
                slots.values().map(|slot| slot.fed.len()).sum::<usize>();
            found.dormant_obligations.0 += state.dormant.borrow().len();
        }
        found
    }
    pub(in crate::application::file_dag::program) fn file_state(&self) -> &RuntimeState {
        &self.file_state
    }
    pub(in crate::application::file_dag::program) fn project_state(
        &self,
        id: crate::application::file_dag::derived::owner::PopulationId,
    ) -> std::rc::Rc<RuntimeState> {
        let mut states = self.project_states.borrow_mut();
        if let Some((_, state)) = states.iter().find(|(key, _)| *key == id) {
            return std::rc::Rc::clone(state);
        }
        let state = std::rc::Rc::new(RuntimeState::new(
            crate::application::file_dag::relations::ProducerScope::Project(id),
            Arc::clone(&self.generation),
        ));
        states.push((id, std::rc::Rc::clone(&state)));
        state
    }
    pub(in crate::application::file_dag::program) fn finish(self) -> Result<(), OperatingError> {
        if self
            .file_state
            .slots
            .borrow()
            .values()
            .any(|slot| matches!(slot.state, State::Running))
            || self.project_states.borrow().iter().any(|(_, state)| {
                state
                    .slots
                    .borrow()
                    .values()
                    .any(|slot| matches!(slot.state, State::Running))
            })
        {
            return Err(OperatingError::NotRun);
        }
        let failure = self.cleanup_failure.into_inner();
        drop(self.file_state);
        drop(self.project_states);
        self.owner.release_population();
        failure.map_or(Ok(()), Err)
    }
    pub(in crate::application::file_dag::program) fn logical_counts(
        &self,
        state: &RuntimeState,
        rule: crate::domain::coverage::types::RuleIndex,
    ) -> Result<crate::application::file_dag::program::protocol::types::LogicalCounts, OperatingError>
    {
        self.validate_state(state)?;
        let rule = self
            .program
            .rules()
            .get(rule.0)
            .ok_or(OperatingError::NotRun)?;
        let slots = state.slots.borrow();
        let mut fed = Vec::with_capacity(rule.original_steps.len());
        for origin in rule.original_steps.iter() {
            let origin = origin.as_ref().map_err(Clone::clone)?;
            let region_id = *self
                .program
                .logical_regions()
                .get(origin.logical.0)
                .ok_or(OperatingError::NotRun)?;
            let code = self
                .program
                .regions()
                .get(region_id.0)
                .ok_or(OperatingError::NotRun)?;
            let at = code
                .instructions
                .iter()
                .position(|instruction| instruction.logical == origin.logical)
                .ok_or(OperatingError::NotRun)?;
            let Some(slot) = slots.get(&region_id.0) else {
                fed.push(crate::domain::operator::types::RowsFed::default());
                continue;
            };
            if matches!(slot.state, State::Running | State::Suspended | State::Ready) {
                return Err(OperatingError::NotRun);
            }
            let total = *slot.fed.get(at).ok_or(OperatingError::NotRun)?;
            fed.push(total);
        }
        Ok(
            crate::application::file_dag::program::protocol::types::LogicalCounts {
                fed: fed.into_boxed_slice(),
            },
        )
    }
    /// Producer truth comes from the compiled declaration and this run's actual
    /// source scope. The consumer's requested partition is never an input.
    pub(in crate::application::file_dag::program) fn source_partition(
        &self,
        root: crate::application::file_dag::program::admission::ProducerRoot,
    ) -> Result<Partition, OperatingError> {
        use crate::domain::executable_program::types::{InputSource, PartitionRequirement};
        if !self.owner.matches_generation(&self.generation) || !self.generation_live.is_live() {
            return Err(OperatingError::NotRead);
        }
        let source = match root {
            crate::application::file_dag::program::admission::ProducerRoot::Input(source) => source,
            crate::application::file_dag::program::admission::ProducerRoot::Region(id) => {
                InputSource::Region(id)
            }
        };
        let declaration = self.program.source_partition(source)?;
        match (declaration, &self.partition) {
            (PartitionRequirement::RequestInvariant, _) => Ok(Partition::RequestInvariant),
            (PartitionRequirement::CurrentFile, Partition::File(file)) => {
                Ok(Partition::File(file.clone()))
            }
            (PartitionRequirement::Project, Partition::Project(project)) => {
                Ok(Partition::Project(project.clone()))
            }
            (PartitionRequirement::InputLineage, actual) => Ok(actual.clone()),
            _ => Err(OperatingError::NotRead),
        }
    }
    pub(in crate::application::file_dag::program) fn partition(&self) -> &Partition {
        &self.partition
    }
    /// A prepared consumer obligation can finish once. The whole transitive
    /// cancellation is preflighted before any slot, count or lease changes.
    pub(super) fn release_inputs(
        &self,
        state: &RuntimeState,
        consumer: RegionId,
    ) -> Result<(), OperatingError> {
        #[cfg(test)]
        self.observe_consumer_release(state, consumer, ConsumerReleasePoint::Before);
        let result = self.retire(state, consumer);
        if let Err(error) = &result {
            self.cleanup_failure
                .borrow_mut()
                .get_or_insert(error.clone());
        }
        #[cfg(test)]
        self.observe_consumer_release(state, consumer, ConsumerReleasePoint::After);
        result
    }
    pub(super) fn retire(
        &self,
        state: &RuntimeState,
        consumer: RegionId,
    ) -> Result<(), OperatingError> {
        let mut work = state.retirement.borrow_mut();
        work.reset();
        let mut slots = state.slots.borrow_mut();
        let slot = slots.get(&consumer.0).ok_or(OperatingError::NotRun)?;
        if slot.dependencies_consumed {
            return Err(OperatingError::NotRun);
        }
        work.pending.extend(
            self.program.regions()[consumer.0]
                .dependencies
                .iter()
                .copied(),
        );
        let planned = (|| {
            while let Some(id) = work.pending.pop() {
                self.program
                    .regions()
                    .get(id.0)
                    .ok_or(OperatingError::NotRun)?;
                let slot = slots.get(&id.0);
                let current = if let Some(current) = work.simulated.get(&id) {
                    *current
                } else {
                    work.touched.push(id);
                    match slot {
                        Some(slot) => slot.remaining,
                        None => match state.dormant.borrow().get(&id).copied() {
                            Some(remaining) => remaining,
                            None => state.initial_remaining(&self.program, id)?,
                        },
                    }
                };
                if current == 0 {
                    return Err(OperatingError::NotRun);
                }
                if current == 1 && slot.is_some_and(|slot| matches!(slot.state, State::Running)) {
                    return Ok(Some(id));
                }
                let remaining = current - 1;
                work.simulated.insert(id, remaining);
                if remaining == 0 && !slot.is_some_and(|slot| slot.dependencies_consumed) {
                    work.pending
                        .extend(self.program.regions()[id.0].dependencies.iter().copied());
                }
            }
            Ok(None)
        })();
        match planned {
            Err(error) => {
                work.reset();
                return Err(error);
            }
            Ok(Some(blocker)) => {
                work.reset();
                if Slot::touched(state, &mut slots, &self.program, consumer)?
                    .waiting_on
                    .is_none()
                {
                    Slot::touched(state, &mut slots, &self.program, consumer)?.waiting_on =
                        Some(blocker);
                    work.waiters.entry(blocker).or_default().push(consumer);
                }
                return Ok(());
            }
            Ok(None) => {}
        }
        Slot::touched(state, &mut slots, &self.program, consumer)?.dependencies_consumed = true;
        Slot::touched(state, &mut slots, &self.program, consumer)?.waiting_on = None;
        for at in 0..work.touched.len() {
            let id = work.touched[at];
            let Some(slot) = slots.get_mut(&id.0) else {
                state.dormant.borrow_mut().insert(id, work.simulated[&id]);
                continue;
            };
            slot.remaining = work.simulated[&id];
            if slot.remaining == 0 {
                slot.dependencies_consumed = true;
                slot.waiting_on = None;
                slot.admission = AdmissionProgress::default();
                let old = std::mem::replace(&mut slot.state, State::Retired);
                if matches!(old, State::Published(_)) {
                    self.owner.release_derived_producer(state.scope, id);
                }
                work.retired.push((old, std::mem::take(&mut slot.scratch)));
            }
        }
        work.reset();
        let mut retired = std::mem::take(&mut work.retired);
        drop(slots);
        drop(work);
        // Lease destructors run outside all scheduler-state borrows.
        retired.clear();
        state.retirement.borrow_mut().retired = retired;
        Ok(())
    }
    fn wake_retirements(&self, state: &RuntimeState, producer: RegionId) {
        // Only a blocker transition wakes its exact consumers. Unchanged
        // running ancestors never cause a full queue or graph rescan.
        let waiters = state.retirement.borrow_mut().waiters.remove(&producer);
        let Some(waiters) = waiters else {
            return;
        };
        for consumer in waiters {
            {
                let mut slots = state.slots.borrow_mut();
                let Some(slot) = slots.get_mut(&consumer.0) else {
                    continue;
                };
                if slot.waiting_on != Some(producer) {
                    continue;
                }
                slot.waiting_on = None;
            }
            if let Err(error) = self.release_inputs(state, consumer) {
                self.cleanup_failure.borrow_mut().get_or_insert(error);
            }
        }
    }
    fn cleanup_status(&self) -> Result<(), OperatingError> {
        if let Some(error) = self.cleanup_failure.borrow().as_ref() {
            return Err(error.clone());
        }
        Ok(())
    }
    pub(in crate::application::file_dag::program) fn validate_project(
        &self,
        program: &Arc<ExecutableProgram>,
        required: &crate::application::file_dag::program::admission::RequiredPartitions,
    ) -> Result<(), OperatingError> {
        if Arc::ptr_eq(&self.program, program)
            && matches!(&self.partition, Partition::Project(actual) if actual.same_request(required))
        {
            Ok(())
        } else {
            Err(OperatingError::WrongOperation)
        }
    }
    /// Drives the admission half of this interpreter for every bound root.
    /// Newly discovered dependencies join the same queue; no dependency graph is
    /// walked ahead of the action that actually demands it.
    pub(in crate::application::file_dag::program) fn prepare_roots(
        &self,
        state: &RuntimeState,
        roots: &[RegionId],
        inputs: &mut dyn Inputs,
        kernels: &dyn KernelInvocation,
        completion: &crate::domain::completion::types::completion_policy::value::CompletionPolicy,
        context: &mut OperatorContext<'_>,
    ) -> Result<(), OperatingError> {
        self.validate_state(state)?;
        use crate::domain::readiness::types::{
            ReadinessAction, ReadinessOutcome, RegionCount, ready_queue::value::ReadyQueue,
        };
        // Publication delivery and retries must not re-enter readiness work.
        let mut pending = Vec::with_capacity(roots.len());
        for &root in roots {
            self.program
                .regions()
                .get(root.0)
                .ok_or(OperatingError::NotRun)?;
            let terminal = state.slots.borrow().get(&root.0).is_some_and(|slot| {
                matches!(
                    slot.state,
                    State::Published(_) | State::Failed(_) | State::Retired
                )
            });
            if !terminal && state.dormant.borrow().get(&root) != Some(&0) {
                pending.push(root);
            }
        }
        let mut queue = ReadyQueue::new(RegionCount(self.program.regions().len()), &pending)?;
        let result = (|| {
            while let Some(action) = queue.next()? {
                let (id, prepare) = match action {
                    ReadinessAction::Prepare(id) => (id, true),
                    ReadinessAction::Evaluate(id) => (id, false),
                };
                let outcome =
                    match self.poll(state, id, inputs, kernels, completion, context, prepare) {
                        Ok(InvocationOutcome::Suspended(dependency)) => {
                            ReadinessOutcome::Waiting(dependency)
                        }
                        Ok(InvocationOutcome::Ready) if prepare => ReadinessOutcome::Ready,
                        Ok(InvocationOutcome::Ready) => return Err(OperatingError::NotRun),
                        // A failed producer wakes its dependants so each resumes its
                        // actual admission action, records the same failure and retires.
                        Ok(_) | Err(_) => ReadinessOutcome::Terminal,
                    };
                queue.observe(id, outcome)?;
            }
            Ok(())
        })();
        let counts = queue.counts();
        *self.readiness_counts.borrow_mut() += counts;
        tracing::trace!(target: "ryo::diagnostic::execution", event = "readiness_queue",
            preparations = counts.preparations.0, evaluations = counts.evaluations.0,
            suspensions = counts.suspensions.0, wakeups = counts.wakeups.0);
        result
    }
    pub(in crate::application::file_dag::program) fn readiness_counts(
        &self,
    ) -> crate::domain::readiness::types::ReadinessCounts {
        *self.readiness_counts.borrow()
    }
    pub(in crate::application::file_dag::program) fn published(
        &self,
        state: &RuntimeState,
        id: RegionId,
    ) -> Result<Option<AdmittedInput>, OperatingError> {
        self.validate_state(state)?;
        let slots = state.slots.borrow();
        let Some(slot) = slots.get(&id.0) else {
            self.program
                .regions()
                .get(id.0)
                .ok_or(OperatingError::NotRun)?;
            return if state.dormant.borrow().get(&id) == Some(&0) {
                Err(OperatingError::NotRun)
            } else {
                Ok(None)
            };
        };
        match &slot.state {
            State::Published(input) => Ok(Some(input.clone())),
            State::Failed(error) => Err(error.clone()),
            State::Retired => Err(OperatingError::NotRun),
            _ => Ok(None),
        }
    }
    /// Observation and frontier aliases enter the same dynamic interpreter only
    /// after their caller's completeness gate has authorized this root.
    pub(in crate::application::file_dag::program) fn admit_root(
        &self,
        state: &RuntimeState,
        rule: crate::domain::coverage::types::RuleIndex,
        source: crate::domain::executable_program::types::InputSource,
        inputs: &mut dyn Inputs,
        kernels: &dyn KernelInvocation,
        completion: &crate::domain::completion::types::completion_policy::value::CompletionPolicy,
        context: &mut OperatorContext<'_>,
    ) -> Result<AdmittedInput, OperatingError> {
        self.validate_state(state)?;
        inputs.bind_rule(rule);
        if let Some(dependency) = inputs.dependency(source)? {
            self.invoke(state, dependency, inputs, kernels, completion, context)?;
            inputs.bind_rule(rule);
        }
        let available = inputs.supply(source, &self.partition, context)?;
        inputs.admit(&available, &self.partition, context)
    }
    pub(in crate::application::file_dag::program) fn invoke(
        &self,
        state: &RuntimeState,
        id: RegionId,
        inputs: &mut dyn Inputs,
        kernels: &dyn KernelInvocation,
        completion: &crate::domain::completion::types::completion_policy::value::CompletionPolicy,
        context: &mut OperatorContext<'_>,
    ) -> Result<InvocationOutcome, OperatingError> {
        self.validate_state(state)?;
        if inputs.population_scope() != state.scope {
            return Err(OperatingError::NotRead);
        }
        let producer = self
            .program
            .regions()
            .get(id.0)
            .ok_or(OperatingError::NotRun)?
            .producer;
        if state
            .slots
            .borrow()
            .get(&id.0)
            .and_then(|slot| slot.completion.as_ref())
            .is_some_and(|bound| bound != &completion.for_rule(producer))
        {
            return Err(OperatingError::NotRead);
        }
        if state
            .slots
            .borrow()
            .get(&id.0)
            .is_some_and(|slot| matches!(slot.state, State::Running))
        {
            return Err(OperatingError::NotRun);
        }
        if let Some(input) = self.published(state, id)? {
            return Ok(InvocationOutcome::Published(input));
        }
        // Completed publications and semantic failures survive a later cleanup
        // refusal; only new work is blocked. finish still reports that refusal.
        self.cleanup_status()?;
        // Singleton demand is the same root-set scheduler, never a recursive evaluator.
        self.prepare_roots(state, &[id], inputs, kernels, completion, context)?;
        let published = self.published(state, id)?;
        self.cleanup_status()?;
        published
            .map(InvocationOutcome::Published)
            .ok_or(OperatingError::NotRun)
    }
    pub(in crate::application::file_dag::program) fn poll(
        &self,
        state: &RuntimeState,
        id: RegionId,
        inputs: &mut dyn Inputs,
        kernels: &dyn KernelInvocation,
        completion: &crate::domain::completion::types::completion_policy::value::CompletionPolicy,
        context: &mut OperatorContext<'_>,
        prepare_only: bool,
    ) -> Result<InvocationOutcome, OperatingError> {
        self.validate_state(state)?;
        if inputs.population_scope() != state.scope {
            return Err(OperatingError::NotRead);
        }
        let _node_origin = crate::application::file_dag::route_counts::node(id.0);
        self.cleanup_status()?;
        let region = self
            .program
            .regions()
            .get(id.0)
            .ok_or(OperatingError::NotRun)?;
        inputs.bind_rule(region.producer);
        let producer_policy = completion.for_rule(region.producer);
        let completion = &producer_policy;
        // A scope borrow never crosses admission, kernels or downstream callbacks.
        let started;
        let mut scratch = {
            let mut slots = state.slots.borrow_mut();
            let slot = Slot::touched(state, &mut slots, &self.program, id)?;
            match &slot.completion {
                Some(bound) if bound != completion => return Err(OperatingError::NotRead),
                Some(_) => {}
                None => slot.completion = Some(completion.clone()),
            }
            match &slot.state {
                State::Running => return Err(OperatingError::NotRun),
                State::Failed(error) => return Err(error.clone()),
                State::Published(input) => return Ok(InvocationOutcome::Published(input.clone())),
                State::Retired => return Err(OperatingError::NotRun),
                State::Dormant | State::Suspended | State::Ready => {}
            }
            started =
                tracing::enabled!(target: "ryo::diagnostic::execution", tracing::Level::TRACE)
                    .then(std::time::Instant::now);
            let fresh = matches!(slot.state, State::Dormant);
            slot.state = State::Running;
            if slot.scratch.is_empty() {
                slot.scratch
                    .resize_with(region.instructions.len(), KernelScratch::default);
            }
            for (frame, instruction) in slot.scratch.iter_mut().zip(region.instructions.iter()) {
                if let Some(workspace) = frame.workspace.as_mut().filter(|_| fresh) {
                    crate::domain::operator::contract::workspace::begin(
                        &instruction.contract.workspace,
                        crate::domain::operator::contract::types::WorkspaceLifetime::Invocation,
                        &mut workspace.primitive,
                    );
                }
            }
            std::mem::take(&mut slot.scratch)
        };
        let mut progress = std::mem::take(
            &mut Slot::touched(state, &mut state.slots.borrow_mut(), &self.program, id)?.admission,
        );
        progress.waiting = None;
        let mut cleanup = Invocation {
            execution: self,
            state,
            id,
            scratch: Some(scratch),
            terminal: Terminal::Retry,
            admission: None,
            started,
            prepare_only,
        };
        let first = region.instructions.first();
        let mut admitted = progress.inputs.clone();
        let result: Result<InvocationOutcome, InvocationFailure> = (|| {
            let first = first.ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
            let needs = region
                .needs
                .as_ref()
                .map_err(|error| InvocationFailure::Semantic(error.clone()))?;
            let program = &first.demand;
            if completion.region_admission(needs)
                == crate::domain::completion::types::RegionAdmission::Blocked
            {
                let last = region
                    .instructions
                    .last()
                    .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
                let input = inputs
                    .publish_empty(
                        crate::application::file_dag::program::admission::ProducerRoot::Region(id),
                        &self.partition,
                        &[],
                        last.contract.output_kinds.clone(),
                    )
                    .map_err(InvocationFailure::Semantic)?;
                return Ok(InvocationOutcome::Published(
                    input.with_coverage(completion.blocked_coverage()),
                ));
            }
            if let Some(shape) = region
                .ordered_existence
                .as_ref()
                .filter(|_| !progress.ordered_done)
            {
                progress
                    .supply(
                        inputs,
                        crate::domain::executable_program::types::InputSource::Region(shape.source),
                        &self.partition,
                        context,
                    )
                    .map_err(InvocationFailure::Semantic)?;
                progress
                    .check_schema(region)
                    .map_err(InvocationFailure::Semantic)?;
                let source = progress
                    .admit(
                        inputs,
                        crate::domain::executable_program::types::InputSource::Region(shape.source),
                        &self.partition,
                        context,
                    )
                    .map_err(InvocationFailure::Semantic)?;
                let permit = KernelPermit::ordered(
                    KernelAdmission {
                        execution: self,
                        region: id,
                    },
                    &source,
                )
                .map_err(InvocationFailure::Semantic)?;
                let last = region
                    .instructions
                    .last()
                    .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
                let mut rows = ryo_values::domain::rows::types::Table::default();
                ryo_values::domain::rows::service::reset(
                    &mut rows,
                    last.contract
                        .output_kinds
                        .as_ref()
                        .map_err(|error| InvocationFailure::Semantic(error.clone()))?,
                );
                let frames = cleanup
                    .scratch
                    .as_mut()
                    .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
                let frame = frames
                    .first_mut()
                    .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
                let outcome = kernels.ordered(
                    shape,
                    &permit,
                    &source,
                    context,
                    frame,
                    &mut crate::domain::replay::types::Collect(&mut rows),
                )?;
                if matches!(outcome, StrategyOutcome::Executed) {
                    completion.filter_rows(
                        region
                            .last_operation()
                            .map_err(InvocationFailure::Semantic)?,
                        needs,
                        &mut rows,
                    );
                    let coverage = source.coverage().map_err(InvocationFailure::Semantic)?;
                    let publication = inputs
                        .publish(
                            crate::application::file_dag::program::admission::ProducerRoot::Region(
                                id,
                            ),
                            &self.partition,
                            &[source],
                            rows,
                        )
                        .map_err(InvocationFailure::Semantic)?
                        .with_coverage(coverage)
                        .with_coverage(completion.coverage(needs));

                    return Ok(InvocationOutcome::Published(publication));
                }
            }
            progress.ordered_done = true;
            if let Some(shape) = region.column.as_ref().filter(|_| !progress.column_done) {
                use crate::domain::executable_program::types::InputSource;
                use crate::domain::executable_program::types::Instruction;
                progress
                    .supply(
                        inputs,
                        InputSource::Region(shape.left()),
                        &self.partition,
                        context,
                    )
                    .map_err(InvocationFailure::Semantic)?;
                progress
                    .supply(
                        inputs,
                        InputSource::Relation(shape.chain().right),
                        &self.partition,
                        context,
                    )
                    .map_err(InvocationFailure::Semantic)?;
                progress
                    .check_schema(region)
                    .map_err(InvocationFailure::Semantic)?;
                let left = progress
                    .admit(
                        inputs,
                        InputSource::Region(shape.left()),
                        &self.partition,
                        context,
                    )
                    .map_err(InvocationFailure::Semantic)?;
                let right = progress
                    .admit(
                        inputs,
                        InputSource::Relation(shape.chain().right),
                        &self.partition,
                        context,
                    )
                    .map_err(InvocationFailure::Semantic)?;
                let permit = KernelPermit::column(
                    KernelAdmission {
                        execution: self,
                        region: id,
                    },
                    &left,
                    &right,
                )
                .map_err(InvocationFailure::Semantic)?;
                let frames = cleanup
                    .scratch
                    .as_mut()
                    .ok_or(InvocationFailure::Semantic(OperatingError::NotRun))?;
                let scratch = frames
                    .first_mut()
                    .ok_or(InvocationFailure::Semantic(OperatingError::NotRun))?;
                let counters = ColumnCounters::default();
                let mut rows = ryo_values::domain::rows::types::Table::default();
                let outcome = kernels.column(
                    shape, &permit, &left, &right, context, scratch, &counters, &mut rows,
                );
                self.record_column(state, shape, &counters)
                    .map_err(InvocationFailure::Semantic)?;
                if matches!(outcome?, StrategyOutcome::Executed) {
                    completion.filter_rows(
                        region
                            .last_operation()
                            .map_err(InvocationFailure::Semantic)?,
                        needs,
                        &mut rows,
                    );
                    let publication = inputs
                        .publish(
                            crate::application::file_dag::program::admission::ProducerRoot::Region(
                                id,
                            ),
                            &self.partition,
                            &[left, right],
                            rows,
                        )
                        .map_err(InvocationFailure::Semantic)?
                        .with_coverage(completion.coverage(needs));
                    return Ok(InvocationOutcome::Published(publication));
                }
            }
            progress.column_done = true;
            if schema_first(region) {
                for action in program.actions().iter().skip(progress.availability_action) {
                    match *action {
                        DemandAction::Admit(InputId(at)) => {
                            let source =
                                *region.inputs.get(at).ok_or(InvocationFailure::Semantic(
                                    OperatingError::MissingInput { position: at + 1 },
                                ))?;
                            progress
                                .supply(inputs, source, &self.partition, context)
                                .map_err(InvocationFailure::Semantic)?;
                        }
                    }
                    progress.availability_action += 1;
                }
                progress
                    .check_schema(region)
                    .map_err(InvocationFailure::Semantic)?;
            }
            for action in program.actions().iter().skip(progress.action) {
                match *action {
                    DemandAction::Admit(InputId(at)) => {
                        let source = *region.inputs.get(at).ok_or_else(|| {
                            InvocationFailure::Semantic(OperatingError::MissingInput {
                                position: at + 1,
                            })
                        })?;
                        admitted.push(
                            progress
                                .admit(inputs, source, &self.partition, context)
                                .map_err(InvocationFailure::Semantic)?,
                        );
                    }
                }
                progress.action += 1;
            }
            if !progress.preflight_done {
                for gate in &first.contract.empty_results {
                    let input = admitted.get(gate.input).ok_or(InvocationFailure::Semantic(
                        OperatingError::MissingInput {
                            position: gate.input + 1,
                        },
                    ))?;
                    if progress
                        .is_empty(
                            inputs,
                            *region
                                .inputs
                                .get(gate.input)
                                .ok_or(InvocationFailure::Semantic(
                                    OperatingError::MissingInput {
                                        position: gate.input + 1,
                                    },
                                ))?,
                            input,
                            gate.representation,
                        )
                        .map_err(InvocationFailure::Semantic)?
                    {
                        let publication = inputs
                            .publish_empty(
                                crate::application::file_dag::program::admission::ProducerRoot::Region(id),
                                &self.partition,
                                &admitted,
                                empty_output_kinds(region, gate),
                            )
                            .map_err(InvocationFailure::Semantic)?;
                        return Ok(InvocationOutcome::Published(
                            publication.with_coverage(completion.coverage(needs)),
                        ));
                    }
                }
            }
            progress
                .check_schema(region)
                .map_err(InvocationFailure::Semantic)?;
            progress.preflight_done = true;
            let permit = KernelPermit::admitted(
                KernelAdmission {
                    execution: self,
                    region: id,
                },
                &admitted,
            )
            .map_err(InvocationFailure::Semantic)?;
            if !progress.ready {
                kernels
                    .ready(&ReadySelection { permit: &permit }, &admitted, context)
                    .map_err(InvocationFailure::Semantic)?;
                progress.ready = true;
            }
            if prepare_only {
                return Ok(InvocationOutcome::Ready);
            }
            let frames = cleanup
                .scratch
                .as_mut()
                .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
            let last = region
                .instructions
                .last()
                .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
            if region.native_positions.is_some() {
                if completion.row_retention()
                    == crate::domain::completion::types::RowRetention::Unnecessary
                {
                    let frame = frames
                        .first_mut()
                        .ok_or(InvocationFailure::Semantic(OperatingError::NotRun))?;
                    if let Some(produced) = kernels.positions(&permit, context, frame)? {
                        let publication = inputs
                            .publish_positions(
                                crate::application::file_dag::program::admission::ProducerRoot::Region(id),
                                &self.partition,
                                &admitted,
                                produced,
                            )
                            .map_err(InvocationFailure::Semantic)?
                            .with_coverage(completion.coverage(needs));
                        return Ok(InvocationOutcome::Published(publication));
                    }
                }
            }
            if let Some(shape) = &region.selection {
                let frame = frames
                    .first_mut()
                    .ok_or(InvocationFailure::Semantic(OperatingError::NotRun))?;
                if let Some(produced) = kernels.selected(shape, &permit, context, frame)? {
                    let publication = inputs
                        .publish_selected(
                            crate::application::file_dag::program::admission::ProducerRoot::Region(
                                id,
                            ),
                            &self.partition,
                            &admitted,
                            produced,
                        )
                        .map_err(InvocationFailure::Semantic)?
                        .with_coverage(completion.coverage(needs));
                    return Ok(InvocationOutcome::Published(publication));
                }
            }
            let mut rows = ryo_values::domain::rows::types::Table::default();
            let output_kinds = &last.contract.output_kinds;
            if let Ok(kinds) = output_kinds {
                ryo_values::domain::rows::service::reset(&mut rows, kinds);
            }
            continuation(
                &permit,
                region,
                0,
                KernelInput::Admitted(&admitted),
                kernels,
                context,
                frames,
                &mut crate::domain::replay::types::Collect(&mut rows),
            )?;
            output_kinds
                .as_ref()
                .map_err(|error| InvocationFailure::Semantic(error.clone()))?;
            completion.filter_rows(
                region
                    .last_operation()
                    .map_err(InvocationFailure::Semantic)?,
                needs,
                &mut rows,
            );
            let published = inputs
                .publish(
                    crate::application::file_dag::program::admission::ProducerRoot::Region(id),
                    &self.partition,
                    &admitted,
                    rows,
                )
                .map_err(InvocationFailure::Semantic)?;
            let published = if completion.observation_admission(needs)
                == crate::domain::completion::types::ObservationAdmission::Ready
            {
                published
            } else {
                published.with_coverage(completion.coverage(needs))
            };
            Ok(InvocationOutcome::Published(published))
        })();
        progress.inputs = admitted;
        cleanup.admission = Some(progress);
        let result = match cleanup.admission.as_ref().and_then(|state| state.waiting) {
            Some(dependency) => Ok(InvocationOutcome::Suspended(dependency)),
            None => result,
        };
        let outcome = match result {
            Ok(InvocationOutcome::Suspended(dependency)) => {
                cleanup.terminal = Terminal::Suspended;
                Ok(InvocationOutcome::Suspended(dependency))
            }
            Ok(InvocationOutcome::Ready) => {
                cleanup.terminal = Terminal::Ready;
                Ok(InvocationOutcome::Ready)
            }
            Ok(InvocationOutcome::Published(input)) => {
                cleanup.terminal = Terminal::Published(input.clone());
                self.release_inputs(state, id)?;
                Ok(InvocationOutcome::Published(input))
            }
            Err(InvocationFailure::Semantic(error) | InvocationFailure::Delivery(error)) => {
                if let Some(admission) = cleanup.admission.as_ref() {
                    tracing::trace!(target: "ryo::diagnostic::execution", event = "region_failure", region = id.0, producer = region.producer.0, phase = ?region.phase, logical = ?first.map(|first| first.logical.0), action = admission.action, ?error);
                }
                cleanup.terminal = Terminal::Failed(error.clone());
                let _cleanup = self.release_inputs(state, id);
                Err(error)
            }
        };
        drop(cleanup);
        match outcome {
            Err(error) => Err(error),
            Ok(outcome) => {
                self.cleanup_status()?;
                Ok(outcome)
            }
        }
    }
}
fn continuation(
    invocation: &KernelPermit<'_>,
    region: &Region,
    at: usize,
    input: KernelInput<'_>,
    kernels: &dyn KernelInvocation,
    context: &mut OperatorContext<'_>,
    scratch: &mut [KernelScratch],
    output: &mut dyn Emit,
) -> Result<(), InvocationFailure> {
    let instruction = region
        .instructions
        .get(at)
        .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
    let permit = invocation
        .instruction(at)
        .map_err(InvocationFailure::Semantic)?;
    let (frame, rest) = scratch
        .split_first_mut()
        .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
    if at + 1 == region.instructions.len() {
        return kernels.evaluate(instruction, &permit, input, context, frame, output);
    }
    let mut next = Continue {
        invocation,
        region,
        at: at + 1,
        kernels,
        scratch: rest,
        output,
    };
    kernels.evaluate(instruction, &permit, input, context, frame, &mut next)
}
impl Emit for Continue<'_> {
    fn row(
        &mut self,
        context: &mut OperatorContext<'_>,
        row: &[Value],
    ) -> Result<(), OperatingError> {
        continuation(
            self.invocation,
            self.region,
            self.at,
            KernelInput::Continuation(row),
            self.kernels,
            context,
            self.scratch,
            self.output,
        )
        .map_err(|failure| match failure {
            InvocationFailure::Semantic(error) | InvocationFailure::Delivery(error) => error,
        })
    }
}
impl Drop for Invocation<'_> {
    fn drop(&mut self) {
        let mut slots = self.state.slots.borrow_mut();
        let mut committed_outcome = "missing";
        if let Some(slot) = slots.get_mut(&self.id.0) {
            slot.scratch = self.scratch.take().unwrap_or_default();
            slot.admission = match self.terminal {
                Terminal::Suspended | Terminal::Ready => self.admission.take().unwrap_or_default(),
                _ => AdmissionProgress::default(),
            };
            for (fed, scratch) in slot.fed.iter_mut().zip(&mut slot.scratch) {
                if let Some(workspace) = &mut scratch.workspace {
                    #[cfg(test)]
                    {
                        slot.observed = workspace.measured();
                        slot.native_positions +=
                            std::mem::take(&mut workspace.observation.native_positions);
                        slot.registered_calls +=
                            std::mem::take(&mut workspace.observation.registered_calls);
                    }
                    for (total, delta) in fed.0.iter_mut().zip(workspace.work.take()) {
                        *total = total.saturating_add(delta);
                    }
                }
            }
            slot.state = match &self.terminal {
                Terminal::Retry => State::Dormant,
                Terminal::Suspended => State::Suspended,
                Terminal::Ready => State::Ready,
                Terminal::Published(input) => match input.publication(&self.execution.owner) {
                    Ok(publication) => {
                        self.execution.owner.pin_derived(publication.as_ref());
                        State::Published(input.clone())
                    }
                    Err(error) => {
                        self.execution
                            .cleanup_failure
                            .borrow_mut()
                            .get_or_insert(error.clone());
                        State::Failed(error)
                    }
                },
                Terminal::Failed(error) => State::Failed(error.clone()),
            };
            committed_outcome = match slot.state {
                State::Dormant => "retry",
                State::Suspended => "suspended",
                State::Ready => "ready",
                State::Running => "running",
                State::Published(_) => "published",
                State::Retired => "retired",
                State::Failed(_) => "failed",
            };
        }
        drop(slots);
        self.execution.wake_retirements(self.state, self.id);
        if let (Some(started), Some(region)) = (
            self.started,
            self.execution.program.regions().get(self.id.0),
        ) {
            // Each segment excludes scheduler wait and cached publication reads.
            // The committed slot outcome is not a claim about the caller's final result.
            tracing::trace!(target: "ryo::diagnostic::execution", event = "region_poll", region = self.id.0,
                producer = region.producer.0, phase = ?region.phase,
                logical = ?region.instructions.first().map(|instruction| instruction.logical.0),
                operation = ?region.instructions.first().map(|instruction| ryo_values::domain::plans::service::kind_of(&instruction.operation)),
                instructions = region.instructions.len(), prepare_only = self.prepare_only,
                committed_outcome, elapsed_ns = started.elapsed().as_nanos() as u64);
        }
    }
}

fn empty_output_kinds(
    region: &Region,
    gate: &crate::domain::operator::contract::types::EmptyResult,
) -> Result<Vec<ryo_relations::domain::relations::types::CellKind>, OperatingError> {
    let mut kinds = gate.kinds.clone()?;
    for instruction in region.instructions.iter().skip(1) {
        kinds = crate::domain::operation_schema::projection::continuation_kinds(
            &instruction.operation,
            &kinds,
        )?;
    }
    Ok(kinds)
}
impl<'scope> KernelPermit<'scope> {
    fn code(
        execution: &ProgramExecution,
        region: RegionId,
    ) -> Result<&crate::domain::executable_program::types::Region, OperatingError> {
        if !execution.owner.matches_generation(&execution.generation) {
            return Err(OperatingError::NotRead);
        }
        let code = execution
            .program
            .regions()
            .get(region.0)
            .ok_or(OperatingError::NotRun)?;
        if code.instructions.is_empty() {
            return Err(OperatingError::NotRun);
        }
        Ok(code)
    }
    pub(in crate::application::file_dag::program) fn admitted(
        authority: KernelAdmission<'scope>,
        inputs: &'scope [AdmittedInput],
    ) -> Result<Self, OperatingError> {
        let execution = authority.execution();
        let region = authority.region();
        let code = Self::code(execution, region)?;
        if code.instructions.is_empty() || inputs.len() != code.inputs.len() {
            return Err(OperatingError::WrongOperation);
        }
        for (source, input) in code.inputs.iter().zip(inputs) {
            let expected = match source {
                InputSource::Region(id) => ProducerRoot::Region(*id),
                other => ProducerRoot::Input(*other),
            };
            input.validate_for(
                &execution.owner,
                &execution.generation,
                execution.partition(),
                expected,
            )?;
        }
        Ok(Self {
            program: &execution.program,
            region,
            instruction: 0,
            generation: &execution.generation,
            owner: execution.owner.as_ref(),
            partition: execution.partition(),
            live: &execution.generation_live,
            bindings: BoundInputs::Ordinary(inputs),
            strategy: KernelStrategy::Registered,
        })
    }
    pub(in crate::application::file_dag::program) fn ordered(
        authority: KernelAdmission<'scope>,
        input: &'scope AdmittedInput,
    ) -> Result<Self, OperatingError> {
        let execution = authority.execution();
        let region = authority.region();
        let code = Self::code(execution, region)?;
        let shape = code
            .ordered_existence
            .as_ref()
            .ok_or(OperatingError::WrongOperation)?;
        input.validate_for(
            &execution.owner,
            &execution.generation,
            execution.partition(),
            ProducerRoot::Region(shape.source),
        )?;
        Ok(Self {
            program: &execution.program,
            region,
            instruction: 0,
            generation: &execution.generation,
            owner: execution.owner.as_ref(),
            partition: execution.partition(),
            live: &execution.generation_live,
            bindings: BoundInputs::Ordered(input),
            strategy: KernelStrategy::Ordered,
        })
    }
    pub(in crate::application::file_dag::program) fn column(
        authority: KernelAdmission<'scope>,
        left: &'scope AdmittedInput,
        right: &'scope AdmittedInput,
    ) -> Result<Self, OperatingError> {
        let execution = authority.execution();
        let region = authority.region();
        let code = Self::code(execution, region)?;
        let shape = code.column.as_ref().ok_or(OperatingError::WrongOperation)?;
        left.validate_for(
            &execution.owner,
            &execution.generation,
            execution.partition(),
            ProducerRoot::Region(shape.left()),
        )?;
        right.validate_for(
            &execution.owner,
            &execution.generation,
            execution.partition(),
            ProducerRoot::Input(InputSource::Relation(shape.chain().right)),
        )?;
        Ok(Self {
            program: &execution.program,
            region,
            instruction: 0,
            generation: &execution.generation,
            owner: execution.owner.as_ref(),
            partition: execution.partition(),
            live: &execution.generation_live,
            bindings: BoundInputs::Column { left, right },
            strategy: KernelStrategy::Column,
        })
    }
    /// A continuation borrows the already admitted invocation; no refcount,
    /// population walk, room or ledger operation occurs here.
    pub(in crate::application::file_dag::program) fn instruction(
        &self,
        at: usize,
    ) -> Result<Self, OperatingError> {
        self.program.regions()[self.region.0]
            .instructions
            .get(at)
            .ok_or(OperatingError::NotRun)?;
        Ok(Self {
            program: self.program,
            region: self.region,
            instruction: at,
            generation: self.generation,
            owner: self.owner,
            partition: self.partition,
            live: self.live,
            bindings: self.bindings,
            strategy: self.strategy,
        })
    }
    pub(in crate::application::file_dag::program) fn native_shape(
        &self,
    ) -> Result<&Arc<crate::domain::lowering::types::LoweredShape>, OperatingError> {
        self.program.regions()[self.region.0]
            .native_positions
            .as_ref()
            .ok_or(OperatingError::WrongOperation)
    }
    pub(in crate::application::file_dag::program) fn region(&self) -> RegionId {
        self.region
    }
    pub(in crate::application::file_dag::program) fn validate_program(
        &self,
        program: &crate::domain::executable_program::types::ExecutableProgram,
    ) -> Result<(), OperatingError> {
        self.validate_live()?;
        if std::ptr::eq(self.program, program) {
            Ok(())
        } else {
            Err(OperatingError::WrongOperation)
        }
    }
    pub(in crate::application::file_dag::program) fn descriptor(
        &self,
    ) -> &crate::domain::executable_program::types::Instruction {
        &self.program.regions()[self.region.0].instructions[self.instruction]
    }
    pub(in crate::application::file_dag::program) fn position_semantics(
        &self,
        shape: &Arc<crate::domain::lowering::types::LoweredShape>,
    ) -> Result<SemanticPermit<'_>, OperatingError> {
        self.validate_registered(self.operation())?;
        let actual = self.program.regions()[self.region.0]
            .native_positions
            .as_ref()
            .ok_or(OperatingError::WrongOperation)?;
        if !Arc::ptr_eq(actual, shape) {
            return Err(OperatingError::WrongOperation);
        }
        let mut permit = self.semantics();
        permit.strategy = KernelStrategy::Positions;
        Ok(permit)
    }
    /// Derived acquisition must borrow the owner of this actual live invocation.
    pub(in crate::application::file_dag) fn validate_derived_owner(
        &self,
        owner: &RelationOwner,
    ) -> Result<(), OperatingError> {
        self.validate_live()?;
        if !std::ptr::eq(self.owner, owner) || !owner.matches_generation(self.generation) {
            return Err(OperatingError::NotRead);
        }
        Ok(())
    }
    pub(in crate::application::file_dag) fn semantics(&self) -> SemanticPermit<'_> {
        SemanticPermit {
            program: self.program,
            region: self.region,
            instruction: self.instruction,
            live: self.live,
            strategy: self.strategy,
        }
    }
    pub(in crate::application::file_dag::program) fn bound_input(
        &self,
        at: usize,
    ) -> Result<&AdmittedInput, OperatingError> {
        match self.bindings {
            BoundInputs::Ordinary(inputs) => inputs.get(at),
            BoundInputs::Ordered(input) => (at == 0).then_some(input),
            BoundInputs::Column { left, right } => match at {
                0 => Some(left),
                1 => Some(right),
                _ => None,
            },
        }
        .ok_or(OperatingError::MissingInput { position: at + 1 })
    }
    pub(in crate::application::file_dag::program) fn validate_live(
        &self,
    ) -> Result<(), OperatingError> {
        if self.live.is_live() {
            Ok(())
        } else {
            Err(OperatingError::NotRead)
        }
    }
    pub(in crate::application::file_dag::program) fn require_file_walk(
        &self,
        expected: &FileGeneration,
    ) -> Result<(), OperatingError> {
        self.validate_live()?;
        match self.partition {
            Partition::File(actual) if actual.matches(expected, self.generation) => Ok(()),
            _ => Err(OperatingError::NotRead),
        }
    }
    pub(in crate::application::file_dag::program) fn require_project_walk(
        &self,
    ) -> Result<(), OperatingError> {
        self.validate_live()?;
        match self.partition {
            Partition::Project(_) => Ok(()),
            _ => Err(OperatingError::NotRead),
        }
    }
    pub(in crate::application::file_dag::program) fn admit_walk_source<'call>(
        &'call self,
        relation: ryo_relations::domain::relations::types::Relation,
        rows: &'call crate::domain::table_model::types::Rows<'call>,
    ) -> Result<Option<AdmittedWalkSource<'call>>, OperatingError> {
        self.validate_live()?;
        let source = self.bound_input(0)?;
        use ryo_values::domain::{
            plan_parts::types::{Input, RepeatedRows},
            plans::types::Operation,
        };
        let authored = match &self.descriptor().operation {
            Operation::Select {
                from: Input::Relation(authored),
                ..
            }
            | Operation::Project {
                from: Input::Relation(authored),
                distinct: RepeatedRows::Kept,
                ..
            } => authored,
            _ => return Err(OperatingError::WrongOperation),
        };
        if *authored != relation {
            return Err(OperatingError::WrongOperation);
        }
        source.walk_source(relation, rows, self.generation)
    }

    pub(in crate::application::file_dag::program) fn validate_walk_source(
        &self,
        source: &AdmittedWalkSource<'_>,
    ) -> Result<(), OperatingError> {
        self.validate_live()?;
        source.require_input(self.bound_input(0)?)
    }
    pub(in crate::application::file_dag::program) fn selection_input<'a>(
        &'a self,
        shape: &crate::domain::executable_program::selection::SelectionAlternative,
    ) -> Result<&'a AdmittedInput, OperatingError> {
        self.validate_live()?;
        let region = &self.program.regions()[self.region.0];
        let declared = region
            .selection
            .as_ref()
            .ok_or(OperatingError::WrongOperation)?;
        if shape.target() != self.region
            || shape.logical() != region.instructions[self.instruction].logical
            || !Arc::ptr_eq(shape.selector(), declared.selector())
        {
            return Err(OperatingError::NotRead);
        }
        let source = self.bound_input(0)?;
        source.validate_publication()?;
        Ok(source)
    }
    pub(in crate::application::file_dag::program) fn output_kinds(
        &self,
    ) -> Result<&[ryo_relations::domain::relations::types::CellKind], OperatingError> {
        self.program.regions()[self.region.0].instructions[self.instruction]
            .contract
            .output_kinds
            .as_deref()
            .map_err(Clone::clone)
    }
    pub(in crate::application::file_dag::program) fn validate_native_join(
        &self,
        native: &crate::application::file_dag::invocation::NativeJoinLease<'_>,
        owner: &Rc<RelationOwner>,
        left: &crate::domain::table_model::types::Rows<'_>,
    ) -> Result<(), OperatingError> {
        self.validate_registered(self.operation())?;
        if !matches!(
            self.native_join_eligibility(),
            crate::application::file_dag::program::native_join::Eligibility::Native
        ) {
            return Err(OperatingError::WrongOperation);
        }
        let region = &self.program.regions()[self.region.0];
        if region.instructions.len() != 1 || region.inputs.len() != 2 {
            return Err(OperatingError::WrongOperation);
        }
        let left_input = self.bound_input(0)?;
        let right_input = self.bound_input(1)?;
        let expected = region.instructions[self.instruction].contract.derived.slots
            [ryo_derived::domain::contract::types::DerivedKind::NativeLookup.ordinal()]
        .as_ref()
        .ok_or(OperatingError::WrongOperation)?;
        if !std::ptr::eq(expected.primary.plan.as_ref(), native.contract()) {
            return Err(OperatingError::NotRead);
        }
        owner.admit(
            left_input.publication(owner)?.as_ref(),
            crate::domain::operator::contract::views::BorrowedRows::from(left),
        )?;
        owner.admit(
            right_input.publication(owner)?.as_ref(),
            crate::domain::operator::contract::views::BorrowedRows::Tree(native.tree()),
        )?;
        native.finding()?;
        Ok(())
    }
    pub(in crate::application::file_dag::program) fn native_join_eligibility(
        &self,
    ) -> crate::application::file_dag::program::native_join::Eligibility {
        let region = &self.program.regions()[self.region.0];
        if region.instructions.len() == 1
            && matches!(
                self.operation(),
                ryo_values::domain::plans::types::Operation::Join { reach: None, filter, .. } if filter.is_empty()
            )
            && region.instructions[self.instruction]
                .contract
                .schema
                .is_ok()
            && region.instructions[self.instruction]
                .contract
                .row_error
                .is_none()
        {
            crate::application::file_dag::program::native_join::Eligibility::Native
        } else {
            crate::application::file_dag::program::native_join::Eligibility::Ordinary
        }
    }
    pub(in crate::application::file_dag) fn validate_registered(
        &self,
        operation: &ryo_values::domain::plans::types::Operation,
    ) -> Result<(), OperatingError> {
        self.validate_live()?;
        if !matches!(self.strategy, KernelStrategy::Registered) || self.operation() != operation {
            return Err(OperatingError::WrongOperation);
        }
        Ok(())
    }
    pub(crate) fn kind(&self) -> ryo_values::domain::plans::types::OperationKind {
        self.program.regions()[self.region.0].instructions[self.instruction]
            .contract
            .kind
    }
    pub(crate) fn operation(&self) -> &ryo_values::domain::plans::types::Operation {
        &self.program.regions()[self.region.0].instructions[self.instruction].operation
    }
}
impl<'scope> SemanticPermit<'scope> {
    fn operation(&self) -> &ryo_values::domain::plans::types::Operation {
        &self.program.regions()[self.region.0].instructions[self.instruction].operation
    }
    fn validate_live(&self) -> Result<(), OperatingError> {
        if self.live.is_live() {
            Ok(())
        } else {
            Err(OperatingError::NotRead)
        }
    }
    pub(in crate::application::file_dag) fn physical<'call, 'meaning>(
        &'call self,
        binding: &crate::domain::executable_program::physical::PhysicalBinding<'_>,
        meaning: ryo_operators::inbound::use_cases::operators::interface::OperatorCall<'meaning>,
    ) -> Result<PermittedPhysical<'call, 'meaning, 'scope>, OperatingError> {
        self.validate_live()?;
        use crate::domain::executable_program::physical::{self, PhysicalRoute};
        let route = match self.strategy {
            KernelStrategy::Registered => PhysicalRoute::Registered,
            KernelStrategy::Positions => PhysicalRoute::Positions(
                self.program.regions()[self.region.0]
                    .native_positions
                    .as_ref()
                    .ok_or(OperatingError::WrongOperation)?,
            ),
            KernelStrategy::Ordered => PhysicalRoute::Ordered(
                self.program.regions()[self.region.0]
                    .ordered_existence
                    .as_ref()
                    .ok_or(OperatingError::WrongOperation)?,
            ),
            KernelStrategy::Column => PhysicalRoute::Column(
                self.program.regions()[self.region.0]
                    .column
                    .as_ref()
                    .ok_or(OperatingError::WrongOperation)?,
            ),
        };
        let authority = physical::authorize(self.operation(), route, binding, &meaning)?;
        Ok(PermittedPhysical {
            permit: self,
            meaning,
            authority,
        })
    }
}
impl PermittedPhysical<'_, '_, '_> {
    pub(in crate::application::file_dag) fn dispatch(
        self,
        port: &dyn crate::outbound::operators::interface::OperatorsPort,
    ) -> Result<
        ryo_operators::inbound::use_cases::operators::interface::OperatorAnswer,
        crate::application::file_dag::executor::KernelFault,
    > {
        if !self.permit.live.is_live() {
            return Err(
                crate::application::file_dag::executor::KernelFault::Refused(
                    OperatingError::NotRead,
                ),
            );
        }
        let _authority = self.authority;
        port.operators(self.meaning)
            .map_err(crate::application::file_dag::executor::KernelFault::Portable)
    }
}

// Private-state proof lives beneath the issuer, never behind concept-wide record fields.
#[cfg(test)]
mod lifecycle;
