//! One region invocation: the poll, its continuation chain and the committed outcome.
use super::super::{
    AdmissionProgress, Continue, Invocation, InvocationOutcome, KernelInvocation, KernelPermit,
    Poll, ProgramExecution, RuntimeState, Slot, State, Terminal,
};
use crate::application::file_dag::program::admission::Inputs;
use crate::application::file_dag::program::constants::EXECUTION_TARGET;
use crate::application::file_dag::program::constants::OUTCOME_FAILED;
use crate::application::file_dag::program::constants::OUTCOME_MISSING;
use crate::application::file_dag::program::constants::OUTCOME_PUBLISHED;
use crate::application::file_dag::program::constants::OUTCOME_READY;
use crate::application::file_dag::program::constants::OUTCOME_RETIRED;
use crate::application::file_dag::program::constants::OUTCOME_RETRY;
use crate::application::file_dag::program::constants::OUTCOME_RUNNING;
use crate::application::file_dag::program::constants::OUTCOME_SUSPENDED;
use crate::application::file_dag::program::constants::REGION_FAILURE;
use crate::application::file_dag::program::constants::REGION_POLL;
use crate::application::file_dag::program::runtime::types::{
    InstructionIndex, InvocationFailure, KernelInput, KernelScratch, PollMode,
};
use ryo_execution_domain::domain::executable_program::types::region::value::Region;
use ryo_execution_domain::domain::executable_program::types::RegionId;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::types::Emit;
use ryo_execution_domain::domain::operating::types::OperatorContext;
use ryo_relations::domain::relations::types::Value;

impl ProgramExecution {
    pub(crate) fn poll(
        &self,
        state: &RuntimeState,
        id: RegionId,
        inputs: &mut dyn Inputs,
        kernels: &dyn KernelInvocation,
        completion: &ryo_execution_domain::domain::completion::types::completion_policy::value::CompletionPolicy,
        context: &mut OperatorContext<'_>,
        prepare_only: PollMode,
    ) -> Result<InvocationOutcome, OperatingError> {
        self.validate_state(state)?;
        if inputs.population_scope() != state.scope {
            return Err(OperatingError::NotRead);
        }
        let _node_origin =
            crate::application::file_dag::route_counts::node(self.owner.routes(), id.0);
        self.cleanup_status()?;
        let region = self
            .program
            .regions()
            .get(id.0)
            .ok_or(OperatingError::NotRun)?;
        inputs.bind_rule(region.producer());
        let producer_policy = completion.for_rule(region.producer());
        let completion = &producer_policy;
        // A scope borrow never crosses admission, kernels or downstream callbacks.
        let started;
        let scratch = {
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
            started = tracing::enabled!(target: EXECUTION_TARGET, tracing::Level::TRACE)
                .then(std::time::Instant::now);
            let fresh = matches!(slot.state, State::Dormant);
            slot.state = State::Running;
            if slot.scratch.is_empty() {
                slot.scratch
                    .resize_with(region.instructions().len(), KernelScratch::default);
            }
            for (frame, instruction) in slot.scratch.iter_mut().zip(region.instructions().iter()) {
                if let Some(workspace) = frame.workspace.as_mut().filter(|_| fresh) {
                    ryo_execution_domain::domain::operator::contract::workspace::begin(
                        &instruction.contract().workspace,
                        ryo_execution_domain::domain::operator::contract::types::WorkspaceLifetime::Invocation,
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
        let first = region.instructions().first();
        let mut admitted = progress.inputs.clone();
        let result: Result<InvocationOutcome, InvocationFailure> = (|| {
            let mut call = Poll {
                id,
                region,
                completion,
                kernels,
                prepare_only,
                inputs,
                context,
                progress: &mut progress,
                frames: &mut cleanup.scratch,
            };
            if let Some(outcome) = self.blocked(&mut call)? {
                return Ok(outcome);
            }
            if let Some(outcome) = self.ordered(&mut call)? {
                return Ok(outcome);
            }
            if let Some(outcome) = self.admit_demand(&mut call, &mut admitted)? {
                return Ok(outcome);
            }
            self.evaluate(&mut call, &admitted)
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
            Err(InvocationFailure::Semantic(error)) => {
                if let Some(admission) = cleanup.admission.as_ref() {
                    tracing::trace!(target: EXECUTION_TARGET, event = REGION_FAILURE, region = id.0, producer = region.producer().0, phase = ?region.phase(), logical = ?first.map(|first| first.logical().0), action = admission.action.0, ?error);
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
pub(crate) fn continuation(
    invocation: &KernelPermit<'_>,
    region: &Region,
    at: InstructionIndex,
    input: KernelInput<'_>,
    kernels: &dyn KernelInvocation,
    context: &mut OperatorContext<'_>,
    scratch: &mut [KernelScratch],
    output: &mut dyn Emit,
) -> Result<(), InvocationFailure> {
    let instruction = region
        .instructions()
        .get(at.0)
        .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
    let permit = invocation
        .instruction(at)
        .map_err(InvocationFailure::Semantic)?;
    let (frame, rest) = scratch
        .split_first_mut()
        .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
    if at.0 + 1 == region.instructions().len() {
        return kernels.evaluate(instruction, &permit, input, context, frame, output);
    }
    let mut next = Continue {
        invocation,
        region,
        at: InstructionIndex(at.0 + 1),
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
            InvocationFailure::Semantic(error) => error,
        })
    }
}
impl Drop for Invocation<'_> {
    fn drop(&mut self) {
        let mut slots = self.state.slots.borrow_mut();
        let mut committed_outcome = OUTCOME_MISSING;
        if let Some(slot) = slots.get_mut(&self.id) {
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
                        slot.registered_calls.0 +=
                            std::mem::take(&mut workspace.observation.registered_calls);
                    }
                    for (total, delta) in fed.0.iter_mut().zip(workspace.work.take().0) {
                        *total = total.saturating_add(delta.0);
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
                State::Dormant => OUTCOME_RETRY,
                State::Suspended => OUTCOME_SUSPENDED,
                State::Ready => OUTCOME_READY,
                State::Running => OUTCOME_RUNNING,
                State::Published(_) => OUTCOME_PUBLISHED,
                State::Retired => OUTCOME_RETIRED,
                State::Failed(_) => OUTCOME_FAILED,
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
            tracing::trace!(target: EXECUTION_TARGET, event = REGION_POLL, region = self.id.0,
                producer = region.producer().0, phase = ?region.phase(),
                logical = ?region.instructions().first().map(|instruction| instruction.logical().0),
                operation = ?region.instructions().first().map(|instruction| ryo_values::domain::plans::service::kind_of(&instruction.operation())),
                instructions = region.instructions().len(), prepare_only = ?self.prepare_only,
                committed_outcome, elapsed_ns = started.elapsed().as_nanos() as u64);
        }
    }
}
