//! Root-set scheduler: prepare, demand, publish lookup and observation roots.
use super::super::{InvocationOutcome, KernelInvocation, ProgramExecution, RuntimeState, State};
use crate::application::file_dag::program::admission::AdmittedInput;
use crate::application::file_dag::program::admission::Inputs;
use crate::application::file_dag::program::constants::EXECUTION_TARGET;
use crate::application::file_dag::program::constants::READINESS_QUEUE;
use crate::domain::executable_program::types::RegionId;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;

impl ProgramExecution {
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
        tracing::trace!(target: EXECUTION_TARGET, event = READINESS_QUEUE,
            preparations = counts.preparations.0, evaluations = counts.evaluations.0,
            suspensions = counts.suspensions.0, wakeups = counts.wakeups.0);
        result
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
}
