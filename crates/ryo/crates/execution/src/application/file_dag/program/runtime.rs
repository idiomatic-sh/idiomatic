//! Sole region invocation lifecycle. Internal continuations have no publication,
//! room, dependency ledger or separate scheduler state.
use super::admission::{AdmittedInput, Inputs, Partition};
use crate::domain::{
    executable_program::types::{
        DemandAction, ExecutableProgram, InputId, Instruction, Region, RegionId,
    },
    operating::{
        errors::OperatingError,
        types::{Emit, OperatorContext},
    },
};
use ryo_relations::domain::relations::types::Value;
use std::{cell::RefCell, sync::Arc};

enum Retirement<'a> {
    Consumer(RegionId),
    Roots(&'a [RegionId]),
}
pub(super) enum Delivery<'a> {
    Retain,
    Forward(crate::domain::coverage::types::RuleIndex, &'a mut dyn Emit),
}
pub(super) enum InvocationFailure {
    Semantic(OperatingError),
    Delivery(OperatingError),
}
/// No held inputs/publication or dependency accounting at internal instructions.
#[derive(Default)]
pub(super) struct KernelScratch {
    pub positions: crate::domain::native_positions::types::PositionWork,
    pub workspace: Option<super::super::workspace::InvocationWorkspace>,
    pub ordered_row: Vec<Value>,
    pub ordered: ryo_operators::inbound::use_cases::operators::interface::OrderedExistenceScratch,
}
pub(super) enum KernelInput<'a> {
    Admitted(
        &'a [AdmittedInput],
        crate::domain::operator::contract::types::EvaluationMode,
    ),
    Continuation(&'a [Value]),
}
#[derive(Default)]
pub(super) struct ColumnCounters {
    pub join: crate::domain::operator::contract::types::WorkCounters,
    pub filter: crate::domain::operator::contract::types::WorkCounters,
    pub project: crate::domain::operator::contract::types::WorkCounters,
}
/// Only the completed admission interpreter can issue a ready selection head.
pub(super) struct ReadySelection<'call, 'scope> {
    permit: &'call super::admission::KernelPermit<'scope>,
}
impl ReadySelection<'_, '_> {
    pub(super) fn permit(&self) -> &super::admission::KernelPermit<'_> {
        self.permit
    }
}
/// Application-owned preparation/acquisition supplies the borrowed semantic call.
/// All implementations are required; no missing-capability or fallback path.
pub(super) trait KernelInvocation {
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
        permit: &super::admission::KernelPermit<'_>,
        input: &AdmittedInput,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
        output: &mut dyn Emit,
    ) -> Result<StrategyOutcome, InvocationFailure>;
    fn positions(
        &self,
        permit: &super::admission::KernelPermit<'_>,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
    ) -> Result<Option<crate::application::file_dag::relations::PublishedInput>, InvocationFailure>;
    fn selected(
        &self,
        shape: &crate::domain::executable_program::selection::SelectionAlternative,
        permit: &super::admission::KernelPermit<'_>,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
    ) -> Result<Option<crate::application::file_dag::relations::PublishedInput>, InvocationFailure>;
    fn column(
        &self,
        shape: &crate::domain::executable_program::column::ColumnAlternative,
        permit: &super::admission::KernelPermit<'_>,
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
        permit: &super::admission::KernelPermit<'_>,
        input: KernelInput<'_>,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
        output: &mut dyn Emit,
    ) -> Result<(), InvocationFailure>;
}
pub(super) enum StrategyOutcome {
    Executed,
    Declined,
}
#[derive(Default)]
struct AdmissionProgress {
    action: usize,
    ordered_done: bool,
    column_done: bool,
    ready: bool,
    preflight_done: bool,
    empty_forwarded: Option<crate::domain::completion::types::CoverageState>,
    inputs: Vec<AdmittedInput>,
    acquired: Vec<(
        crate::domain::executable_program::types::InputSource,
        AdmittedInput,
    )>,
    waiting: Option<RegionId>,
    emptiness: Vec<(
        crate::domain::executable_program::types::InputSource,
        crate::domain::operator::contract::types::EmptyRepresentation,
        bool,
    )>,
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

    fn admit(
        &mut self,
        inputs: &mut dyn Inputs,
        source: crate::domain::executable_program::types::InputSource,
        partition: &Partition,
        context: &mut OperatorContext<'_>,
    ) -> Result<AdmittedInput, OperatingError> {
        if let Some((_, input)) = self.acquired.iter().find(|(key, _)| *key == source) {
            return Ok(input.clone());
        }
        if let Some(dependency) = inputs.dependency(source)? {
            self.waiting = Some(dependency);
            return Err(OperatingError::NotRun);
        }
        let admitted = inputs.admit(source, partition, context)?;
        self.acquired.push((source, admitted.clone()));
        Ok(admitted)
    }
}
enum State {
    Dormant,
    Suspended,
    Ready,
    Running,
    Published(AdmittedInput),
    Forwarded,
    Retired,
    Failed(OperatingError),
}
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AttemptCheckpoint {
    Dormant,
    Running,
    Published,
    Terminal,
}
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DependencyCheckpoint {
    Pending,
    Consumed,
}
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ReaderCount(pub(super) usize);
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RetryCheckpoint {
    pub(super) attempt: AttemptCheckpoint,
    pub(super) remaining: ReaderCount,
    pub(super) dependencies: DependencyCheckpoint,
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
    mode: Option<crate::domain::operator::contract::types::EvaluationMode>,
    failed_prefix: Option<AdmittedInput>,
    prefix_delivered: Vec<crate::domain::coverage::types::RuleIndex>,
    forward_consumers: Vec<crate::domain::coverage::types::RuleIndex>,
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
    simulated: Vec<usize>,
    touched: Vec<RegionId>,
    pending: Vec<RegionId>,
    waiters: std::collections::HashMap<RegionId, Vec<RegionId>>,
    retired: Vec<(State, Vec<KernelScratch>)>,
}
impl RetirementWork {
    fn reset(&mut self) {
        for id in self.touched.drain(..) {
            self.simulated[id.0] = usize::MAX;
        }
        self.pending.clear();
    }
}
pub(crate) struct ProgramExecution {
    pub(super) program: Arc<ExecutableProgram>,
    pub(super) owner: std::rc::Rc<super::super::relations::RelationOwner>,
    pub(super) generation: Arc<()>,
    pub(super) generation_live: Arc<super::super::relations::GenerationLease>,
    partition: Partition,
    slots: RefCell<Vec<Slot>>,
    retirement: RefCell<RetirementWork>,
    cleanup_failure: RefCell<Option<OperatingError>>,
}
/// Constructors are private to the outer execution owner; no primitive can open
/// another scope or choose a scheduler.
impl ProgramExecution {
    pub(in crate::application::file_dag::program) fn open(
        program: Arc<ExecutableProgram>,
        partition: Partition,
        owner: std::rc::Rc<super::super::relations::RelationOwner>,
    ) -> Self {
        let slots = program
            .regions()
            .iter()
            .map(|region| Slot {
                #[cfg(test)]
                observed: Default::default(),
                #[cfg(test)]
                native_positions: 0,
                #[cfg(test)]
                registered_calls: 0,
                state: State::Dormant,
                scratch: Vec::new(),
                admission: AdmissionProgress::default(),
                mode: None,
                failed_prefix: None,
                prefix_delivered: Vec::new(),
                forward_consumers: Vec::new(),
                fed: vec![
                    crate::domain::operator::types::RowsFed::default();
                    region.instructions.len()
                ]
                .into_boxed_slice(),
                remaining: region.consumers,
                dependencies_consumed: false,
                waiting_on: None,
                completion: None,
            })
            .collect();
        let region_count = program.regions().len();
        let generation = owner.generation_guard();
        let generation_live = owner.generation_liveness();
        Self {
            program,
            owner,
            generation,
            generation_live,
            partition,
            retirement: RefCell::new(RetirementWork {
                simulated: vec![usize::MAX; region_count],
                touched: Vec::new(),
                pending: Vec::new(),
                waiters: std::collections::HashMap::new(),
                retired: Vec::new(),
            }),
            slots: RefCell::new(slots),
            cleanup_failure: RefCell::new(None),
        }
    }
    fn record_column(
        &self,
        shape: &crate::domain::executable_program::column::ColumnAlternative,
        counts: &ColumnCounters,
    ) -> Result<(), OperatingError> {
        let mut slots = self.slots.borrow_mut();
        for (id, values) in [
            (shape.join(), counts.join.snapshot()),
            (shape.filter(), counts.filter.snapshot()),
            (shape.target(), counts.project.snapshot()),
        ] {
            let slot = slots.get_mut(id.0).ok_or(OperatingError::NotRun)?;
            let fed = slot.fed.first_mut().ok_or(OperatingError::NotRun)?;
            for (total, found) in fed.0.iter_mut().zip(values) {
                *total = total.saturating_add(found);
            }
        }
        Ok(())
    }
    /// Observes completed dispatches even after the real retirement drops invocation scratch.
    #[cfg(test)]
    pub(super) fn route_observation(
        &self,
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
        self.region_observation(*region)
    }
    #[cfg(test)]
    pub(super) fn region_observation(
        &self,
        region: RegionId,
    ) -> Result<crate::domain::operating::types::ExecutionObservation, OperatingError> {
        let slots = self.slots.borrow();
        let slot = slots.get(region.0).ok_or(OperatingError::NotRun)?;
        Ok(crate::domain::operating::types::ExecutionObservation {
            native_positions: slot.native_positions,
            registered_calls: slot.registered_calls,
            ..slot.observed
        })
    }
    #[cfg(test)]
    pub(super) fn retry_checkpoint(&self, id: RegionId) -> Result<RetryCheckpoint, OperatingError> {
        let slots = self.slots.borrow();
        let slot = slots.get(id.0).ok_or(OperatingError::NotRun)?;
        Ok(RetryCheckpoint {
            attempt: match slot.state {
                State::Dormant => AttemptCheckpoint::Dormant,
                State::Running | State::Suspended | State::Ready => AttemptCheckpoint::Running,
                State::Published(_) => AttemptCheckpoint::Published,
                State::Forwarded | State::Retired | State::Failed(_) => AttemptCheckpoint::Terminal,
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
    pub(super) fn finish(self) -> Result<(), OperatingError> {
        let slots = self.slots.into_inner();
        if slots
            .iter()
            .any(|slot| matches!(slot.state, State::Running))
        {
            return Err(OperatingError::NotRun);
        }
        let failure = self.cleanup_failure.into_inner();
        drop(slots);
        self.owner.release_population();
        match failure {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
    pub(super) fn logical_counts(
        &self,
        rule: crate::domain::coverage::types::RuleIndex,
    ) -> Result<super::facade::LogicalCounts, OperatingError> {
        let rule = self
            .program
            .rules()
            .get(rule.0)
            .ok_or(OperatingError::NotRun)?;
        let slots = self.slots.borrow();
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
            let slot = slots.get(region_id.0).ok_or(OperatingError::NotRun)?;
            if matches!(slot.state, State::Running | State::Suspended | State::Ready) {
                return Err(OperatingError::NotRun);
            }
            let mut total = *slot.fed.get(at).ok_or(OperatingError::NotRun)?;
            // Both terminal roles retain the same authored logical coordinates.
            // Count actual work from either or both attempts without changing
            // the original-step identity used by frontier provenance/digests.
            if let Ok(diagnostic) = &rule.forward_emit {
                let diagnostic_code = self
                    .program
                    .regions()
                    .get(diagnostic.0)
                    .ok_or(OperatingError::NotRun)?;
                if let Some(at) = diagnostic_code
                    .instructions
                    .iter()
                    .position(|instruction| instruction.logical == origin.logical)
                {
                    let attempt = slots.get(diagnostic.0).ok_or(OperatingError::NotRun)?;
                    if matches!(attempt.state, State::Running | State::Suspended) {
                        return Err(OperatingError::NotRun);
                    }
                    for (total, next) in total
                        .0
                        .iter_mut()
                        .zip(attempt.fed.get(at).ok_or(OperatingError::NotRun)?.0)
                    {
                        *total = total.saturating_add(next);
                    }
                }
            }
            fed.push(total);
        }
        Ok(super::facade::LogicalCounts {
            fed: fed.into_boxed_slice(),
        })
    }
    /// Producer truth comes from the compiled declaration and this run's actual
    /// source scope. The consumer's requested partition is never an input.
    pub(super) fn source_partition(
        &self,
        root: super::admission::ProducerRoot,
    ) -> Result<Partition, OperatingError> {
        use crate::domain::executable_program::types::{InputSource, PartitionRequirement};
        if !self.owner.matches_generation(&self.generation) || !self.generation_live.is_live() {
            return Err(OperatingError::NotRead);
        }
        let source = match root {
            super::admission::ProducerRoot::Input(source) => source,
            super::admission::ProducerRoot::Region(id) => InputSource::Region(id),
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
    pub(super) fn partition(&self) -> &Partition {
        &self.partition
    }
    /// A prepared consumer obligation can finish once. The whole transitive
    /// cancellation is preflighted before any slot, count or lease changes.
    fn release_inputs(&self, consumer: RegionId) -> Result<(), OperatingError> {
        let result = self.retire(Retirement::Consumer(consumer));
        if let Err(error) = &result {
            self.cleanup_failure
                .borrow_mut()
                .get_or_insert(error.clone());
        }
        result
    }
    pub(super) fn cancel_roots(&self, roots: &[RegionId]) -> Result<(), OperatingError> {
        self.retire(Retirement::Roots(roots))
    }
    fn retire(&self, retirement: Retirement<'_>) -> Result<(), OperatingError> {
        let mut work = self.retirement.borrow_mut();
        work.reset();
        let mut slots = self.slots.borrow_mut();
        match retirement {
            Retirement::Consumer(consumer) => {
                let slot = slots.get(consumer.0).ok_or(OperatingError::NotRun)?;
                if slot.dependencies_consumed {
                    return Err(OperatingError::NotRun);
                }
                work.pending.extend(
                    self.program.regions()[consumer.0]
                        .dependencies
                        .iter()
                        .copied(),
                );
            }
            Retirement::Roots(roots) => work.pending.extend(roots.iter().copied()),
        }
        let planned = (|| {
            while let Some(id) = work.pending.pop() {
                let slot = slots.get(id.0).ok_or(OperatingError::NotRun)?;
                let current = if work.simulated[id.0] == usize::MAX {
                    work.touched.push(id);
                    slot.remaining
                } else {
                    work.simulated[id.0]
                };
                if current == 0 {
                    return Err(OperatingError::NotRun);
                }
                if current == 1 && matches!(slot.state, State::Running) {
                    return Ok(Some(id));
                }
                let remaining = current - 1;
                work.simulated[id.0] = remaining;
                if remaining == 0 && !slot.dependencies_consumed {
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
                let Retirement::Consumer(consumer) = retirement else {
                    // File-level cancellation cannot race an active invocation.
                    return Err(OperatingError::NotRun);
                };
                if slots[consumer.0].waiting_on.is_none() {
                    slots[consumer.0].waiting_on = Some(blocker);
                    work.waiters.entry(blocker).or_default().push(consumer);
                }
                return Ok(());
            }
            Ok(None) => {}
        }
        if let Retirement::Consumer(consumer) = retirement {
            slots[consumer.0].dependencies_consumed = true;
            slots[consumer.0].waiting_on = None;
        }
        for at in 0..work.touched.len() {
            let id = work.touched[at];
            let slot = &mut slots[id.0];
            slot.remaining = work.simulated[id.0];
            if slot.remaining == 0 {
                slot.dependencies_consumed = true;
                slot.waiting_on = None;
                slot.admission = AdmissionProgress::default();
                slot.failed_prefix = None;
                slot.prefix_delivered.clear();
                let old = std::mem::replace(&mut slot.state, State::Retired);
                if matches!(old, State::Published(_)) {
                    self.owner.release_derived_producer(id);
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
        self.retirement.borrow_mut().retired = retired;
        Ok(())
    }
    fn wake_retirements(&self, producer: RegionId) {
        // Only a blocker transition wakes its exact consumers. Unchanged
        // running ancestors never cause a full queue or graph rescan.
        let waiters = self.retirement.borrow_mut().waiters.remove(&producer);
        let Some(waiters) = waiters else {
            return;
        };
        for consumer in waiters {
            {
                let mut slots = self.slots.borrow_mut();
                let slot = &mut slots[consumer.0];
                if slot.waiting_on != Some(producer) {
                    continue;
                }
                slot.waiting_on = None;
            }
            if let Err(error) = self.release_inputs(consumer) {
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
    pub(super) fn project_partition_matches(
        &self,
        required: &super::admission::RequiredPartitions,
    ) -> bool {
        matches!(&self.partition, Partition::Project(actual) if actual.same_request(required))
    }
    pub(in crate::application::file_dag::program) fn declare_forwarder(
        &self,
        id: RegionId,
        rule: crate::domain::coverage::types::RuleIndex,
    ) -> Result<(), OperatingError> {
        let mut slots = self.slots.borrow_mut();
        let slot = slots.get_mut(id.0).ok_or(OperatingError::NotRun)?;
        if !matches!(slot.state, State::Dormant) {
            return Err(OperatingError::NotRead);
        }
        if !slot.forward_consumers.contains(&rule) {
            slot.forward_consumers.push(rule);
        }
        Ok(())
    }
    /// Drives the admission half of this interpreter for every bound root.
    /// Newly discovered dependencies join the same queue; no dependency graph is
    /// walked ahead of the action that actually demands it.
    pub(in crate::application::file_dag::program) fn prepare_roots(
        &self,
        roots: &[(
            RegionId,
            crate::domain::operator::contract::types::EvaluationMode,
        )],
        inputs: &mut dyn Inputs,
        kernels: &dyn KernelInvocation,
        completion: &crate::domain::completion::types::completion_policy::value::CompletionPolicy,
        context: &mut OperatorContext<'_>,
    ) -> Result<(), OperatingError> {
        use crate::domain::operator::contract::types::EvaluationMode;
        {
            let mut slots = self.slots.borrow_mut();
            for (id, mode) in roots {
                let slot = slots.get_mut(id.0).ok_or(OperatingError::NotRun)?;
                match slot.mode {
                    Some(bound) if bound != *mode => return Err(OperatingError::NotRead),
                    _ => slot.mode = Some(*mode),
                }
            }
        }
        let mut pending = roots.to_vec();
        let mut known: std::collections::HashSet<_> = roots.iter().map(|(id, _)| *id).collect();
        let mut ready = Vec::new();
        let mut waiting = std::collections::HashMap::new();
        while !pending.is_empty() {
            let mut blocked = Vec::new();
            let mut discovered = Vec::new();
            let mut terminal_progress = false;
            for (id, mode) in pending.drain(..) {
                match self.poll(
                    id,
                    inputs,
                    kernels,
                    completion,
                    context,
                    Delivery::Retain,
                    Some(mode),
                ) {
                    Ok(InvocationOutcome::Suspended(dependency)) => {
                        waiting.insert(id, dependency);
                        let mut cursor = dependency;
                        let mut seen = std::collections::HashSet::new();
                        while let Some(next) = waiting.get(&cursor).copied() {
                            if cursor == id || !seen.insert(cursor) {
                                return Err(OperatingError::NotRun);
                            }
                            cursor = next;
                        }
                        blocked.push((id, mode));
                        if known.insert(dependency) {
                            let mode = self.slots.borrow()[dependency.0]
                                .mode
                                .unwrap_or(EvaluationMode::Collected);
                            discovered.push((dependency, mode));
                        }
                    }
                    Ok(InvocationOutcome::Ready) => {
                        waiting.remove(&id);
                        if !ready.iter().any(|(found, _)| *found == id) {
                            ready.push((id, mode));
                        }
                    }
                    // A terminal source refusal belongs to its own root. Other
                    // independent roots still reach their actual admission head.
                    Ok(_) | Err(_) => {
                        waiting.remove(&id);
                        terminal_progress = true;
                    }
                }
            }
            if !discovered.is_empty() {
                discovered.extend(blocked);
                pending = discovered;
                continue;
            }
            // Every currently reachable head has submitted its exact class.
            // Resume collected work, keeping forwarded roots parked for their
            // caller's sink. A callback borrow never enters this queue.
            let mut advanced = terminal_progress;
            for (id, mode) in ready.drain(..) {
                if mode == EvaluationMode::Collected
                    || self.program.regions()[id.0].publishes
                    || waiting.values().any(|dependency| *dependency == id)
                {
                    let _outcome =
                        self.invoke(id, inputs, kernels, completion, context, Delivery::Retain);
                    advanced = true;
                }
            }
            if blocked.is_empty() {
                break;
            }
            if !advanced {
                return Err(OperatingError::NotRun);
            }
            pending = blocked;
        }
        Ok(())
    }
    pub(in crate::application::file_dag::program) fn published(
        &self,
        id: RegionId,
    ) -> Result<Option<AdmittedInput>, OperatingError> {
        let slots = self.slots.borrow();
        match &slots.get(id.0).ok_or(OperatingError::NotRun)?.state {
            State::Published(input) => Ok(Some(input.clone())),
            State::Failed(error) => Err(error.clone()),
            State::Forwarded | State::Retired => Err(OperatingError::NotRun),
            _ => Ok(None),
        }
    }
    /// Observation and frontier aliases enter the same dynamic interpreter only
    /// after their caller's completeness gate has authorized this root.
    pub(in crate::application::file_dag::program) fn admit_root(
        &self,
        rule: crate::domain::coverage::types::RuleIndex,
        source: crate::domain::executable_program::types::InputSource,
        inputs: &mut dyn Inputs,
        kernels: &dyn KernelInvocation,
        completion: &crate::domain::completion::types::completion_policy::value::CompletionPolicy,
        context: &mut OperatorContext<'_>,
    ) -> Result<AdmittedInput, OperatingError> {
        inputs.bind_rule(rule);
        if let Some(dependency) = inputs.dependency(source)? {
            self.invoke(
                dependency,
                inputs,
                kernels,
                completion,
                context,
                Delivery::Retain,
            )?;
            inputs.bind_rule(rule);
        }
        inputs.admit(source, &self.partition, context)
    }
    pub(in crate::application::file_dag::program) fn invoke(
        &self,
        id: RegionId,
        inputs: &mut dyn Inputs,
        kernels: &dyn KernelInvocation,
        completion: &crate::domain::completion::types::completion_policy::value::CompletionPolicy,
        context: &mut OperatorContext<'_>,
        delivery: Delivery<'_>,
    ) -> Result<InvocationOutcome, OperatingError> {
        let mut delivery = delivery;
        loop {
            let next = match &mut delivery {
                Delivery::Retain => Delivery::Retain,
                Delivery::Forward(consumer, output) => Delivery::Forward(*consumer, &mut **output),
            };
            match self.poll(id, inputs, kernels, completion, context, next, None)? {
                InvocationOutcome::Suspended(dependency) => {
                    let mut pending = vec![id, dependency];
                    self.resolve_dependencies(&mut pending, inputs, kernels, completion, context)?;
                }
                InvocationOutcome::Ready => return Err(OperatingError::NotRun),
                outcome => return Ok(outcome),
            }
        }
    }
    fn resolve_dependencies(
        &self,
        pending: &mut Vec<RegionId>,
        inputs: &mut dyn Inputs,
        kernels: &dyn KernelInvocation,
        completion: &crate::domain::completion::types::completion_policy::value::CompletionPolicy,
        context: &mut OperatorContext<'_>,
    ) -> Result<(), OperatingError> {
        while pending.len() > 1 {
            let current = *pending.last().ok_or(OperatingError::NotRun)?;
            match self.poll(
                current,
                inputs,
                kernels,
                completion,
                context,
                Delivery::Retain,
                None,
            ) {
                Ok(InvocationOutcome::Suspended(dependency)) => {
                    if pending.contains(&dependency) {
                        {
                            let mut slots = self.slots.borrow_mut();
                            let slot = slots.get_mut(current.0).ok_or(OperatingError::NotRun)?;
                            slot.state = State::Failed(OperatingError::NotRun);
                            slot.admission = AdmissionProgress::default();
                        }
                        self.release_inputs(current)?;
                        pending.pop();
                        continue;
                    }
                    pending.push(dependency);
                }
                Ok(InvocationOutcome::Ready) => return Err(OperatingError::NotRun),
                // A producer failure is consumed by its waiting parent at the
                // same suspended admission action. Unwind through every parent
                // so each records terminal failure and settles its obligations.
                Ok(_) | Err(_) => {
                    pending.pop();
                }
            }
        }
        Ok(())
    }
    pub(in crate::application::file_dag::program) fn poll(
        &self,
        id: RegionId,
        inputs: &mut dyn Inputs,
        kernels: &dyn KernelInvocation,
        completion: &crate::domain::completion::types::completion_policy::value::CompletionPolicy,
        context: &mut OperatorContext<'_>,
        delivery: Delivery<'_>,
        prepare_mode: Option<crate::domain::operator::contract::types::EvaluationMode>,
    ) -> Result<InvocationOutcome, OperatingError> {
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
        let (mut scratch, bound_mode) = {
            let mut slots = self.slots.borrow_mut();
            let slot = slots.get_mut(id.0).ok_or(OperatingError::NotRun)?;
            match &slot.completion {
                Some(bound) if bound != completion => return Err(OperatingError::NotRead),
                Some(_) => {}
                None => slot.completion = Some(completion.clone()),
            }
            match &slot.state {
                State::Running => return Err(OperatingError::NotRun),
                State::Failed(error) => {
                    let error = error.clone();
                    let consumer = match &delivery {
                        Delivery::Forward(consumer, _) => Some(*consumer),
                        Delivery::Retain => None,
                    };
                    let prefix = if consumer
                        .is_some_and(|consumer| slot.prefix_delivered.contains(&consumer))
                    {
                        None
                    } else {
                        slot.failed_prefix.clone()
                    };
                    drop(slots);
                    if let (Some(prefix), Delivery::Forward(consumer, output)) = (prefix, delivery)
                    {
                        inputs.replay(&prefix, context, output)?;
                        let mut slots = self.slots.borrow_mut();
                        let slot = &mut slots[id.0];
                        slot.prefix_delivered.push(consumer);
                        if !slot.forward_consumers.is_empty()
                            && slot
                                .forward_consumers
                                .iter()
                                .all(|rule| slot.prefix_delivered.contains(rule))
                        {
                            slot.failed_prefix = None;
                        }
                    }
                    return Err(error);
                }
                State::Published(input) => {
                    let input = input.clone();
                    drop(slots);
                    return match delivery {
                        Delivery::Retain => Ok(InvocationOutcome::Published(input)),
                        Delivery::Forward(_, output) => {
                            inputs.replay(&input, context, output)?;
                            Ok(InvocationOutcome::Forwarded(input.coverage()?))
                        }
                    };
                }
                State::Forwarded | State::Retired => return Err(OperatingError::NotRun),
                State::Dormant | State::Suspended | State::Ready => {}
            }
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
            let mode = *slot.mode.get_or_insert_with(|| {
                prepare_mode.unwrap_or_else(|| match &delivery {
                    Delivery::Retain => {
                        crate::domain::operator::contract::types::EvaluationMode::Collected
                    }
                    Delivery::Forward(_, _) => {
                        crate::domain::operator::contract::types::EvaluationMode::Streaming
                    }
                })
            });
            (std::mem::take(&mut slot.scratch), mode)
        };
        let mut progress = std::mem::take(&mut self.slots.borrow_mut()[id.0].admission);
        progress.waiting = None;
        let mut cleanup = Invocation {
            execution: self,
            id,
            scratch: Some(scratch),
            terminal: Terminal::Retry,
            admission: None,
            failed_prefix: None,
            delivered_consumer: None,
        };
        let first = region.instructions.first().ok_or(OperatingError::NotRun)?;
        let needs = region.needs.as_ref().map_err(Clone::clone)?;
        let mut admitted = progress.inputs.clone();
        let evaluation = bound_mode;
        let program = match evaluation {
            crate::domain::operator::contract::types::EvaluationMode::Collected => {
                &first.collected_demand
            }
            crate::domain::operator::contract::types::EvaluationMode::Streaming => {
                &first.streaming_demand
            }
        };
        let after_success = program
            .actions()
            .contains(&DemandAction::PublishAfterSuccess);
        let result: Result<InvocationOutcome, InvocationFailure> = (|| {
            if let Some(coverage) = progress.empty_forwarded.clone() {
                return if prepare_mode.is_some() {
                    Ok(InvocationOutcome::Ready)
                } else {
                    Ok(InvocationOutcome::Forwarded(coverage))
                };
            }
            if completion.region_admission(needs)
                == crate::domain::completion::types::RegionAdmission::Blocked
            {
                let last = region
                    .instructions
                    .last()
                    .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
                let input = inputs
                    .publish_empty(
                        super::admission::ProducerRoot::Region(id),
                        &self.partition,
                        &[],
                        last.contract.collected_kinds.clone(),
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
                let source = progress
                    .admit(
                        inputs,
                        crate::domain::executable_program::types::InputSource::Region(shape.source),
                        &self.partition,
                        context,
                    )
                    .map_err(InvocationFailure::Semantic)?;
                let permit = super::admission::KernelPermit::ordered(self, id, &source, evaluation)
                    .map_err(InvocationFailure::Semantic)?;
                let last = region
                    .instructions
                    .last()
                    .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
                let mut rows = ryo_values::domain::rows::types::Table::default();
                ryo_values::domain::rows::service::reset(
                    &mut rows,
                    last.contract
                        .collected_kinds
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
                            super::admission::ProducerRoot::Region(id),
                            &self.partition,
                            &[source],
                            rows,
                        )
                        .map_err(InvocationFailure::Semantic)?
                        .with_coverage(coverage)
                        .with_coverage(completion.coverage(needs));
                    if let Delivery::Forward(_, output) = delivery {
                        inputs
                            .replay(&publication, context, output)
                            .map_err(InvocationFailure::Delivery)?;
                    }
                    return Ok(InvocationOutcome::Published(publication));
                }
            }
            progress.ordered_done = true;
            if let (
                Some(shape),
                crate::domain::operator::contract::types::EvaluationMode::Collected,
            ) = (
                region.column.as_ref().filter(|_| !progress.column_done),
                evaluation,
            ) {
                use crate::domain::executable_program::types::{HeldJoinOrder, InputSource};
                let (left, right) = match shape.order() {
                    HeldJoinOrder::RightBeforeLeft => {
                        let right = progress
                            .admit(
                                inputs,
                                InputSource::Relation(shape.chain().right),
                                &self.partition,
                                context,
                            )
                            .map_err(InvocationFailure::Semantic)?;
                        let left = progress
                            .admit(
                                inputs,
                                InputSource::Region(shape.left()),
                                &self.partition,
                                context,
                            )
                            .map_err(InvocationFailure::Semantic)?;
                        (left, Some(right))
                    }
                    HeldJoinOrder::LeftWithEmptyGate => {
                        let left = progress
                            .admit(
                                inputs,
                                InputSource::Region(shape.left()),
                                &self.partition,
                                context,
                            )
                            .map_err(InvocationFailure::Semantic)?;
                        let right = if progress
                            .is_empty(
                                inputs,
                                InputSource::Region(shape.left()),
                                &left,
                                crate::domain::operator::contract::types::EmptyRepresentation::Any,
                            )
                            .map_err(InvocationFailure::Semantic)?
                        {
                            None
                        } else {
                            Some(
                                progress
                                    .admit(
                                        inputs,
                                        InputSource::Relation(shape.chain().right),
                                        &self.partition,
                                        context,
                                    )
                                    .map_err(InvocationFailure::Semantic)?,
                            )
                        };
                        (left, right)
                    }
                };
                let Some(right) = right else {
                    let published = inputs
                        .publish_empty(
                            super::admission::ProducerRoot::Region(id),
                            &self.partition,
                            &[left],
                            first.contract.collected_kinds.clone(),
                        )
                        .map_err(InvocationFailure::Semantic)?;
                    return Ok(InvocationOutcome::Published(
                        published.with_coverage(completion.coverage(needs)),
                    ));
                };
                if progress
                    .is_empty(
                        inputs,
                        InputSource::Region(shape.left()),
                        &left,
                        crate::domain::operator::contract::types::EmptyRepresentation::Any,
                    )
                    .map_err(InvocationFailure::Semantic)?
                {
                    let published = inputs
                        .publish_empty(
                            super::admission::ProducerRoot::Region(id),
                            &self.partition,
                            &[left, right],
                            first.contract.collected_kinds.clone(),
                        )
                        .map_err(InvocationFailure::Semantic)?;
                    return Ok(InvocationOutcome::Published(
                        published.with_coverage(completion.coverage(needs)),
                    ));
                }
                let permit = super::admission::KernelPermit::column(self, id, &left, &right)
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
                self.record_column(shape, &counters)
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
                            super::admission::ProducerRoot::Region(id),
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
            for (action_index, action) in program.actions().iter().enumerate().skip(progress.action)
            {
                match *action {
                    DemandAction::ValidateBeforeInputs => {
                        first
                            .contract
                            .schema
                            .as_ref()
                            .map_err(|error| InvocationFailure::Semantic(error.clone()))?;
                    }
                    DemandAction::ValidateFirstRow | DemandAction::PublishAfterSuccess => {}
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
                    DemandAction::EmptyGate(InputId(at)) => {
                        if progress
                            .is_empty(
                                inputs,
                                *region.inputs.get(at).ok_or(InvocationFailure::Semantic(
                                    OperatingError::MissingInput { position: at + 1 },
                                ))?,
                                admitted.get(at).ok_or_else(|| {
                                    InvocationFailure::Semantic(OperatingError::MissingInput {
                                        position: at + 1,
                                    })
                                })?,
                                crate::domain::operator::contract::types::EmptyRepresentation::Any,
                            )
                            .map_err(InvocationFailure::Semantic)?
                        {
                            let published = inputs
                                .publish_empty(
                                    super::admission::ProducerRoot::Region(id),
                                    &self.partition,
                                    &admitted,
                                    region
                                        .instructions
                                        .last()
                                        .ok_or_else(|| {
                                            InvocationFailure::Semantic(OperatingError::NotRun)
                                        })?
                                        .contract
                                        .collected_kinds
                                        .clone(),
                                )
                                .map_err(InvocationFailure::Semantic)?;
                            return Ok(InvocationOutcome::Published(published));
                        }
                    }
                    DemandAction::JoinInputs { .. } => {
                        admitted = admit_join(
                            region,
                            first,
                            inputs,
                            &mut progress,
                            &self.partition,
                            context,
                        )
                        .map_err(InvocationFailure::Semantic)?;
                        if admitted.len() == 1 {
                            // The mandatory left-empty gate ends the invocation before RHS
                            // acquisition and before any semantic permit is issued.
                            let last = region.instructions.last().ok_or_else(|| {
                                InvocationFailure::Semantic(OperatingError::NotRun)
                            })?;
                            let published = inputs
                                .publish_empty(
                                    super::admission::ProducerRoot::Region(id),
                                    &self.partition,
                                    &admitted,
                                    last.contract.collected_kinds.clone(),
                                )
                                .map_err(InvocationFailure::Semantic)?;
                            return Ok(InvocationOutcome::Published(published));
                        }
                    }
                    DemandAction::AxisInputs => {
                        admitted = admit_axis(
                            region,
                            first,
                            inputs,
                            &mut progress,
                            &self.partition,
                            context,
                        )
                        .map_err(InvocationFailure::Semantic)?;
                        if admitted.len() == 1 {
                            // A held empty primary never demands its side populations.
                            if evaluation == crate::domain::operator::contract::types::EvaluationMode::Streaming && !region.publishes {
                                let coverage = completion.coverage(needs);
                                if prepare_mode.is_some() {
                                    progress.empty_forwarded = Some(coverage);
                                    return Ok(InvocationOutcome::Ready);
                                }
                                return Ok(InvocationOutcome::Forwarded(coverage));
                            }
                            let gate = first
                                .contract
                                .collected_empty
                                .iter()
                                .find(|gate| gate.input == 0)
                                .ok_or(InvocationFailure::Semantic(
                                    OperatingError::WrongOperation,
                                ))?;
                            let publication = inputs
                                .publish_empty(
                                    super::admission::ProducerRoot::Region(id),
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
                progress.action = action_index + 1;
                progress.inputs = admitted.clone();
            }
            if !progress.preflight_done
                && evaluation == crate::domain::operator::contract::types::EvaluationMode::Collected
            {
                for gate in &first.contract.collected_empty {
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
                                super::admission::ProducerRoot::Region(id),
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
                for instruction in region.instructions.iter().rev() {
                    if instruction.contract.collected.preflight
                        == crate::domain::operator::contract::types::PreflightOrder::SchemaFirst
                    {
                        instruction
                            .contract
                            .collected_kinds
                            .as_ref()
                            .map_err(|error| InvocationFailure::Semantic(error.clone()))?;
                    }
                }
            }
            progress.preflight_done = true;
            let permit = super::admission::KernelPermit::admitted(self, id, &admitted, evaluation)
                .map_err(InvocationFailure::Semantic)?;
            if !progress.ready {
                kernels
                    .ready(&ReadySelection { permit: &permit }, &admitted, context)
                    .map_err(InvocationFailure::Semantic)?;
                progress.ready = true;
            }
            if prepare_mode.is_some() {
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
            if let (Some(_), crate::domain::operator::contract::types::EvaluationMode::Collected) =
                (&region.native_positions, evaluation)
            {
                if completion.row_retention()
                    == crate::domain::completion::types::RowRetention::Unnecessary
                {
                    let frame = frames
                        .first_mut()
                        .ok_or(InvocationFailure::Semantic(OperatingError::NotRun))?;
                    if let Some(produced) = kernels.positions(&permit, context, frame)? {
                        let publication = inputs
                            .publish_positions(
                                super::admission::ProducerRoot::Region(id),
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
            if let (
                Some(shape),
                crate::domain::operator::contract::types::EvaluationMode::Collected,
            ) = (&region.selection, evaluation)
            {
                let frame = frames
                    .first_mut()
                    .ok_or(InvocationFailure::Semantic(OperatingError::NotRun))?;
                if let Some(produced) = kernels.selected(shape, &permit, context, frame)? {
                    let publication = inputs
                        .publish_selected(
                            super::admission::ProducerRoot::Region(id),
                            &self.partition,
                            &admitted,
                            produced,
                        )
                        .map_err(InvocationFailure::Semantic)?
                        .with_coverage(completion.coverage(needs));
                    return Ok(InvocationOutcome::Published(publication));
                }
            }
            match delivery {
                Delivery::Forward(_, output)
                    if !region.publishes
                        && completion.row_retention()
                            == crate::domain::completion::types::RowRetention::Unnecessary
                        && !after_success =>
                {
                    let mut output = GuardedOutput {
                        output,
                        error: None,
                    };
                    let result = continuation(
                        &permit,
                        region,
                        0,
                        KernelInput::Admitted(&admitted, evaluation),
                        kernels,
                        context,
                        frames,
                        &mut output,
                    );
                    if let Some(error) = output.error {
                        return Err(InvocationFailure::Delivery(error));
                    }
                    result?;
                    let inherited = super::admission::contributor_readiness(
                        &self.partition,
                        &admitted,
                        &self.owner,
                    )
                    .map_err(InvocationFailure::Semantic)?;
                    let inherited = match inherited {
                        super::admission::Readiness::Complete => {
                            crate::domain::completion::types::CoverageState::Complete
                        }
                        super::admission::Readiness::Partial { files } => {
                            crate::domain::completion::types::CoverageState::Partial { files }
                        }
                    };
                    Ok(InvocationOutcome::Forwarded(
                        crate::domain::completion::service::merge_coverage(
                            inherited,
                            completion.coverage(needs),
                        ),
                    ))
                }
                delivery => {
                    let mut rows = ryo_values::domain::rows::types::Table::default();
                    let output_kinds = &last.contract.collected_kinds;
                    if let Ok(kinds) = output_kinds {
                        ryo_values::domain::rows::service::reset(&mut rows, kinds);
                    }
                    match delivery {
                        Delivery::Retain => {
                            let mut output = crate::domain::replay::types::Collect(&mut rows);
                            let result = continuation(
                                &permit,
                                region,
                                0,
                                KernelInput::Admitted(&admitted, evaluation),
                                kernels,
                                context,
                                frames,
                                &mut output,
                            );
                            if result.is_err()
                                && evaluation == crate::domain::operator::contract::types::EvaluationMode::Streaming
                                && !after_success
                                && completion.row_retention() == crate::domain::completion::types::RowRetention::Unnecessary
                            {
                                cleanup.failed_prefix = Some(inputs.publish(
                                    super::admission::ProducerRoot::Region(id), &self.partition,
                                    &admitted, std::mem::take(&mut rows),
                                ).map_err(InvocationFailure::Semantic)?);
                            }
                            result?;
                        }
                        Delivery::Forward(_, output)
                            if completion.row_retention()
                                == crate::domain::completion::types::RowRetention::Required
                                || after_success =>
                        {
                            let mut collect = crate::domain::replay::types::Collect(&mut rows);
                            continuation(
                                &permit,
                                region,
                                0,
                                KernelInput::Admitted(&admitted, evaluation),
                                kernels,
                                context,
                                frames,
                                &mut collect,
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
                            let width = rows.kinds.len();
                            for at in 0..rows.count.0 {
                                let start = at * width;
                                output
                                    .row(context, &rows.cells[start..start + width])
                                    .map_err(InvocationFailure::Delivery)?;
                            }
                        }
                        Delivery::Forward(consumer, output) => {
                            let mut output = Tee {
                                rows: &mut rows,
                                output,
                                error: None,
                            };
                            let result = continuation(
                                &permit,
                                region,
                                0,
                                KernelInput::Admitted(&admitted, evaluation),
                                kernels,
                                context,
                                frames,
                                &mut output,
                            );
                            if let Some(error) = output.error {
                                return Err(InvocationFailure::Delivery(error));
                            }
                            if matches!(&result, Err(InvocationFailure::Semantic(_))) {
                                cleanup.failed_prefix = Some(
                                    inputs
                                        .publish(
                                            super::admission::ProducerRoot::Region(id),
                                            &self.partition,
                                            &admitted,
                                            std::mem::take(&mut rows),
                                        )
                                        .map_err(InvocationFailure::Semantic)?,
                                );
                                cleanup.delivered_consumer = Some(consumer);
                            }
                            result?;
                        }
                    }
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
                            super::admission::ProducerRoot::Region(id),
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
                }
            }
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
                self.release_inputs(id)?;
                Ok(InvocationOutcome::Published(input))
            }
            Ok(InvocationOutcome::Forwarded(coverage)) => {
                cleanup.terminal = Terminal::Forwarded;
                self.release_inputs(id)?;
                Ok(InvocationOutcome::Forwarded(coverage))
            }
            Err(InvocationFailure::Semantic(error)) => {
                cleanup.terminal = Terminal::Failed(error.clone());
                let _cleanup = self.release_inputs(id);
                Err(error)
            }
            Err(InvocationFailure::Delivery(error)) => Err(error),
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
    invocation: &super::admission::KernelPermit<'_>,
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
struct Continue<'a> {
    invocation: &'a super::admission::KernelPermit<'a>,
    region: &'a Region,
    at: usize,
    kernels: &'a dyn KernelInvocation,
    scratch: &'a mut [KernelScratch],
    output: &'a mut dyn Emit,
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
struct GuardedOutput<'a> {
    output: &'a mut dyn Emit,
    error: Option<OperatingError>,
}
impl Emit for GuardedOutput<'_> {
    fn row(
        &mut self,
        context: &mut OperatorContext<'_>,
        row: &[Value],
    ) -> Result<(), OperatingError> {
        let result = self.output.row(context, row);
        if let Err(error) = &result {
            if self.error.is_none() {
                self.error = Some(error.clone());
            }
        }
        result
    }
}
pub(super) enum InvocationOutcome {
    Suspended(RegionId),
    Ready,
    Published(AdmittedInput),
    Forwarded(crate::domain::completion::types::CoverageState),
}
enum Terminal {
    Retry,
    Suspended,
    Ready,
    Published(AdmittedInput),
    Forwarded,
    Failed(OperatingError),
}
struct Invocation<'a> {
    execution: &'a ProgramExecution,
    id: RegionId,
    scratch: Option<Vec<KernelScratch>>,
    terminal: Terminal,
    admission: Option<AdmissionProgress>,
    failed_prefix: Option<AdmittedInput>,
    delivered_consumer: Option<crate::domain::coverage::types::RuleIndex>,
}
impl Drop for Invocation<'_> {
    fn drop(&mut self) {
        let mut slots = self.execution.slots.borrow_mut();
        if let Some(slot) = slots.get_mut(self.id.0) {
            slot.scratch = self.scratch.take().unwrap_or_default();
            if let Some(prefix) = self.failed_prefix.take() {
                slot.failed_prefix = Some(prefix);
            }
            if let Some(consumer) = self.delivered_consumer {
                slot.prefix_delivered.push(consumer);
            }
            if !slot.forward_consumers.is_empty()
                && slot
                    .forward_consumers
                    .iter()
                    .all(|rule| slot.prefix_delivered.contains(rule))
            {
                slot.failed_prefix = None;
            }
            slot.admission = match self.terminal {
                Terminal::Suspended | Terminal::Ready => self.admission.take().unwrap_or_default(),
                _ => AdmissionProgress::default(),
            };
            for (fed, scratch) in slot.fed.iter_mut().zip(&mut slot.scratch) {
                if let Some(workspace) = &mut scratch.workspace {
                    #[cfg(test)]
                    {
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
                Terminal::Forwarded => State::Forwarded,
                Terminal::Failed(error) => State::Failed(error.clone()),
            };
        }
        drop(slots);
        self.execution.wake_retirements(self.id);
    }
}

#[cfg(test)]
mod runtime_lifecycle_test;

fn admit_join(
    region: &Region,
    instruction: &Instruction,
    inputs: &mut dyn Inputs,
    progress: &mut AdmissionProgress,
    partition: &Partition,
    context: &mut OperatorContext<'_>,
) -> Result<Vec<AdmittedInput>, OperatingError> {
    use crate::domain::{executable_program::types::HeldJoinOrder, operating::types::Supply};
    let supply = |at| {
        instruction
            .input_delivery
            .get(at)
            .copied()
            .ok_or(OperatingError::MissingInput { position: at + 1 })
    };
    let source = |at| {
        region
            .inputs
            .get(at)
            .copied()
            .ok_or(OperatingError::MissingInput { position: at + 1 })
    };
    let held = instruction
        .streaming_demand
        .actions()
        .iter()
        .find_map(|action| match action {
            DemandAction::JoinInputs { held } => Some(*held),
            _ => None,
        })
        .ok_or(OperatingError::WrongOperation)?;
    if supply(0)? == Supply::AtHand || supply(1)? == Supply::Streamed {
        match held {
            HeldJoinOrder::RightBeforeLeft => {
                let right = progress.admit(inputs, source(1)?, partition, context)?;
                let left = progress.admit(inputs, source(0)?, partition, context)?;
                Ok(vec![left, right])
            }
            HeldJoinOrder::LeftWithEmptyGate => {
                let left = progress.admit(inputs, source(0)?, partition, context)?;
                if progress.is_empty(
                    inputs,
                    source(0)?,
                    &left,
                    crate::domain::operator::contract::types::EmptyRepresentation::Any,
                )? {
                    return Ok(vec![left]);
                }
                let right = progress.admit(inputs, source(1)?, partition, context)?;
                Ok(vec![left, right])
            }
        }
    } else {
        let right = progress.admit(inputs, source(1)?, partition, context)?;
        let left = progress.admit(inputs, source(0)?, partition, context)?;
        Ok(vec![left, right])
    }
}
fn admit_axis(
    region: &Region,
    instruction: &Instruction,
    inputs: &mut dyn Inputs,
    progress: &mut AdmissionProgress,
    partition: &Partition,
    context: &mut OperatorContext<'_>,
) -> Result<Vec<AdmittedInput>, OperatingError> {
    use crate::domain::operating::types::Supply;
    let mut delivery = Vec::with_capacity(3);
    for input in 0..3 {
        delivery.push(*instruction.input_delivery.get(input).ok_or(
            OperatingError::MissingInput {
                position: input + 1,
            },
        )?);
    }
    let source = |at: usize| {
        region
            .inputs
            .get(at)
            .copied()
            .ok_or(OperatingError::MissingInput { position: at + 1 })
    };
    if delivery.as_slice() == [Supply::Streamed, Supply::AtHand, Supply::AtHand] {
        let edges = progress.admit(inputs, source(1)?, partition, context)?;
        let domain = progress.admit(inputs, source(2)?, partition, context)?;
        let primary = progress.admit(inputs, source(0)?, partition, context)?;
        Ok(vec![primary, edges, domain])
    } else {
        let primary = progress.admit(inputs, source(0)?, partition, context)?;
        if progress.is_empty(
            inputs,
            source(0)?,
            &primary,
            crate::domain::operator::contract::types::EmptyRepresentation::Any,
        )? {
            return Ok(vec![primary]);
        }
        let edges = progress.admit(inputs, source(1)?, partition, context)?;
        let domain = progress.admit(inputs, source(2)?, partition, context)?;
        Ok(vec![primary, edges, domain])
    }
}

struct Tee<'a> {
    rows: &'a mut ryo_values::domain::rows::types::Table,
    output: &'a mut dyn Emit,
    error: Option<OperatingError>,
}
impl Emit for Tee<'_> {
    fn row(
        &mut self,
        context: &mut OperatorContext<'_>,
        row: &[Value],
    ) -> Result<(), OperatingError> {
        self.rows.cells.extend_from_slice(row);
        self.rows.count.0 += 1;
        let result = self.output.row(context, row);
        if let Err(error) = &result {
            if self.error.is_none() {
                self.error = Some(error.clone());
            }
        }
        result
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
