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
    Forward(&'a mut dyn Emit),
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
/// Application-owned preparation/acquisition supplies the borrowed semantic call.
/// All implementations are required; no missing-capability or fallback path.
pub(super) trait KernelInvocation {
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
enum State {
    Dormant,
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
    fed: Box<[crate::domain::operator::types::RowsFed]>,
    remaining: usize,
    dependencies_consumed: bool,
    waiting_on: Option<RegionId>,
    completion: Option<crate::application::file_dag::completion::CompletionPolicy>,
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
                State::Running => AttemptCheckpoint::Running,
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
            if matches!(slot.state, State::Running) {
                return Err(OperatingError::NotRun);
            }
            fed.push(*slot.fed.get(at).ok_or(OperatingError::NotRun)?);
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
    pub(in crate::application::file_dag::program) fn invoke(
        &self,
        id: RegionId,
        inputs: &mut dyn Inputs,
        kernels: &dyn KernelInvocation,
        completion: &crate::application::file_dag::completion::CompletionPolicy,
        context: &mut OperatorContext<'_>,
        delivery: Delivery<'_>,
    ) -> Result<InvocationOutcome, OperatingError> {
        let _node_origin = crate::application::file_dag::route_counts::node(id.0);
        self.cleanup_status()?;
        let region = self
            .program
            .regions()
            .get(id.0)
            .ok_or(OperatingError::NotRun)?;
        let producer_policy = completion.for_rule(region.producer);
        let completion = &producer_policy;
        // A scope borrow never crosses admission, kernels or downstream callbacks.
        let mut scratch = {
            let mut slots = self.slots.borrow_mut();
            let slot = slots.get_mut(id.0).ok_or(OperatingError::NotRun)?;
            match &slot.completion {
                Some(bound) if bound != completion => return Err(OperatingError::NotRead),
                Some(_) => {}
                None => slot.completion = Some(completion.clone()),
            }
            match &slot.state {
                State::Running => return Err(OperatingError::NotRun),
                State::Failed(error) => return Err(error.clone()),
                State::Published(input) => {
                    let input = input.clone();
                    drop(slots);
                    return match delivery {
                        Delivery::Retain => Ok(InvocationOutcome::Published(input)),
                        Delivery::Forward(output) => {
                            inputs.replay(&input, context, output)?;
                            Ok(InvocationOutcome::Forwarded(input.coverage()?))
                        }
                    };
                }
                State::Forwarded | State::Retired => return Err(OperatingError::NotRun),
                State::Dormant => {}
            }
            slot.state = State::Running;
            if slot.scratch.is_empty() {
                slot.scratch
                    .resize_with(region.instructions.len(), KernelScratch::default);
            }
            for (frame, instruction) in slot.scratch.iter_mut().zip(region.instructions.iter()) {
                if let Some(workspace) = frame.workspace.as_mut() {
                    crate::domain::operator::contract::workspace::begin(
                        &instruction.contract.workspace,
                        crate::domain::operator::contract::types::WorkspaceLifetime::Invocation,
                        &mut workspace.primitive,
                    );
                }
            }
            std::mem::take(&mut slot.scratch)
        };
        let mut cleanup = Invocation {
            execution: self,
            id,
            scratch: Some(scratch),
            terminal: Terminal::Retry,
        };
        let first = region.instructions.first().ok_or(OperatingError::NotRun)?;
        let needs = region.needs.as_ref().map_err(Clone::clone)?;
        let mut admitted = Vec::new();
        let evaluation = match &delivery {
            Delivery::Retain => crate::domain::operator::contract::types::EvaluationMode::Collected,
            Delivery::Forward(_) => {
                crate::domain::operator::contract::types::EvaluationMode::Streaming
            }
        };
        let program = match delivery {
            Delivery::Retain => &first.collected_demand,
            Delivery::Forward(_) => &first.streaming_demand,
        };
        let after_success = program.actions().contains(&DemandAction::PublishAfterSuccess);
        let result: Result<InvocationOutcome, InvocationFailure> = (|| {
            if !completion.allows_region(needs) {
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
            if let Some(shape) = &region.ordered_existence {
                let source = inputs
                    .admit(
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
                    if let Delivery::Forward(output) = delivery {
                        inputs
                            .replay(&publication, context, output)
                            .map_err(InvocationFailure::Delivery)?;
                    }
                    return Ok(InvocationOutcome::Published(publication));
                }
            }
            if let (Some(shape), Delivery::Retain) = (&region.column, &delivery) {
                use crate::domain::executable_program::types::{HeldJoinOrder, InputSource};
                let (left, right) = match shape.order() {
                    HeldJoinOrder::RightBeforeLeft => {
                        let right = inputs
                            .admit(
                                InputSource::Relation(shape.chain().right),
                                &self.partition,
                                context,
                            )
                            .map_err(InvocationFailure::Semantic)?;
                        let left = inputs
                            .admit(InputSource::Region(shape.left()), &self.partition, context)
                            .map_err(InvocationFailure::Semantic)?;
                        (left, Some(right))
                    }
                    HeldJoinOrder::LeftWithEmptyGate => {
                        let left = inputs
                            .admit(InputSource::Region(shape.left()), &self.partition, context)
                            .map_err(InvocationFailure::Semantic)?;
                        let right = if inputs
                            .is_empty(
                                &left,
                                crate::domain::operator::contract::types::EmptyRepresentation::Any,
                            )
                            .map_err(InvocationFailure::Semantic)?
                        {
                            None
                        } else {
                            Some(
                                inputs
                                    .admit(
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
                if inputs
                    .is_empty(
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
            for action in program.actions() {
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
                            inputs
                                .admit(source, &self.partition, context)
                                .map_err(InvocationFailure::Semantic)?,
                        );
                    }
                    DemandAction::EmptyGate(InputId(at)) => {
                        if inputs
                            .is_empty(
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
                        admitted = admit_join(region, first, inputs, &self.partition, context)
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
                        admitted = admit_axis(region, first, inputs, &self.partition, context)
                            .map_err(InvocationFailure::Semantic)?;
                        if admitted.len() == 1 {
                            // A held empty primary never demands its side populations.
                            if matches!(delivery, Delivery::Forward(_)) && !region.publishes {
                                return Ok(InvocationOutcome::Forwarded(
                                    completion.coverage(needs),
                                ));
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
            }
            if evaluation == crate::domain::operator::contract::types::EvaluationMode::Collected {
                for gate in &first.contract.collected_empty {
                    let input = admitted.get(gate.input).ok_or(InvocationFailure::Semantic(
                        OperatingError::MissingInput {
                            position: gate.input + 1,
                        },
                    ))?;
                    if inputs
                        .is_empty(input, gate.representation)
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
            let permit = super::admission::KernelPermit::admitted(self, id, &admitted, evaluation)
                .map_err(InvocationFailure::Semantic)?;
            let frames = cleanup
                .scratch
                .as_mut()
                .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
            let last = region
                .instructions
                .last()
                .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
            if let (Some(_), Delivery::Retain) = (&region.native_positions, &delivery) {
                if !completion.requires_buffer() {
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
            if let (Some(shape), Delivery::Retain) = (&region.selection, &delivery) {
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
                Delivery::Forward(output) if !region.publishes && !completion.requires_buffer() && !after_success => {
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
                            crate::application::file_dag::completion::CoverageState::Complete
                        }
                        super::admission::Readiness::Partial { files } => {
                            crate::application::file_dag::completion::CoverageState::Partial {
                                files,
                            }
                        }
                    };
                    Ok(InvocationOutcome::Forwarded(
                        inherited.merge(completion.coverage(needs)),
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
                            continuation(
                                &permit,
                                region,
                                0,
                                KernelInput::Admitted(&admitted, evaluation),
                                kernels,
                                context,
                                frames,
                                &mut output,
                            )?;
                        }
                        Delivery::Forward(output) if completion.requires_buffer() || after_success => {
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
                        Delivery::Forward(output) => {
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
                    let published = if completion.observation_ready(needs) {
                        published
                    } else {
                        published.with_coverage(completion.coverage(needs))
                    };
                    Ok(InvocationOutcome::Published(published))
                }
            }
        })();
        let outcome = match result {
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
    Published(AdmittedInput),
    Forwarded(crate::application::file_dag::completion::CoverageState),
}
enum Terminal {
    Retry,
    Published(AdmittedInput),
    Forwarded,
    Failed(OperatingError),
}
struct Invocation<'a> {
    execution: &'a ProgramExecution,
    id: RegionId,
    scratch: Option<Vec<KernelScratch>>,
    terminal: Terminal,
}
impl Drop for Invocation<'_> {
    fn drop(&mut self) {
        let mut slots = self.execution.slots.borrow_mut();
        if let Some(slot) = slots.get_mut(self.id.0) {
            slot.scratch = self.scratch.take().unwrap_or_default();
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
                let right = inputs.admit(source(1)?, partition, context)?;
                let left = inputs.admit(source(0)?, partition, context)?;
                Ok(vec![left, right])
            }
            HeldJoinOrder::LeftWithEmptyGate => {
                let left = inputs.admit(source(0)?, partition, context)?;
                if inputs.is_empty(
                    &left,
                    crate::domain::operator::contract::types::EmptyRepresentation::Any,
                )? {
                    return Ok(vec![left]);
                }
                let right = inputs.admit(source(1)?, partition, context)?;
                Ok(vec![left, right])
            }
        }
    } else {
        let right = inputs.admit(source(1)?, partition, context)?;
        let left = inputs.admit(source(0)?, partition, context)?;
        Ok(vec![left, right])
    }
}
fn admit_axis(
    region: &Region,
    instruction: &Instruction,
    inputs: &mut dyn Inputs,
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
        let edges = inputs.admit(source(1)?, partition, context)?;
        let domain = inputs.admit(source(2)?, partition, context)?;
        let primary = inputs.admit(source(0)?, partition, context)?;
        Ok(vec![primary, edges, domain])
    } else {
        let primary = inputs.admit(source(0)?, partition, context)?;
        if inputs.is_empty(
            &primary,
            crate::domain::operator::contract::types::EmptyRepresentation::Any,
        )? {
            return Ok(vec![primary]);
        }
        let edges = inputs.admit(source(1)?, partition, context)?;
        let domain = inputs.admit(source(2)?, partition, context)?;
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
