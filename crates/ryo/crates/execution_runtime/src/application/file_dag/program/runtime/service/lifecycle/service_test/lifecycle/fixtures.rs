//! The support of the `lifecycle` cases: one compiled program run through each lifecycle path.
use ryo_operators::domain::table_scratch::types::OperatorScratch;
use super::super::super::fixtures::{
    Reentry, RuntimeProbe, RuntimeProbeMode, Seen, Sink, SinkMode, context,
};
use super::super::super::inputs::fixtures::{CompiledCase, TestInputs, compiled};
use crate::application::file_dag::program::admission::service::Inputs;
use crate::application::file_dag::program::runtime::service::InvocationOutcome;
use crate::application::file_dag::program::runtime::service::Slot;
use crate::application::file_dag::program::runtime::service::State;
use crate::application::file_dag::program::runtime::types::DependencyCheckpoint;
use ryo_execution_domain::domain::completion::types::completion_policy::value::CompletionPolicy;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::{
    coverage::types::RuleIndex,
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::FileId;
use std::rc::Rc;

use ryo_execution_domain::domain::readiness::types::ReadinessCounts;
use std::borrow::Cow;

/// What a compiled program's lifecycle observed.
#[derive(Debug, PartialEq)]
pub(super) enum Lived {
    /// A nested failure: the outer root's two invocations, each root's slot failure and whether it
    /// still holds frames, and the probe's evaluations.
    Nested {
        invocations: [Option<OperatingError>; 2],
        slots: Cow<'static, [(Option<OperatingError>, bool)]>,
        evaluations: usize,
        /// What the probe saw inside the outer frame.
        seen: Cow<'static, [Seen]>,
    },
    /// Same-region reentry: whether each invocation published, what the reentry saw, and the
    /// evaluations.
    Reentered {
        published: [bool; 2],
        reentry: Reentry,
        evaluations: usize,
    },
    /// A sink refusing the first replay: its refusal, whether the root's slot is published and holds
    /// no frames, the rows the retry's replay delivered, and the evaluations.
    Retried {
        refusal: Option<OperatingError>,
        published: bool,
        frames_empty: bool,
        replayed: usize,
        evaluations: usize,
    },
    /// Whether the root published, its evaluations and the readiness counts.
    Counted {
        published: bool,
        evaluations: usize,
        counts: ReadinessCounts,
    },
    /// Reader initializations after 64 touches of one root and after one of another, and the slots.
    Initialized {
        first: usize,
        second: usize,
        slots: usize,
    },
    /// A repeated release beside a primary semantic failure.
    Cleanup(Cleanup),
}

/// A root retained for two external obligations, released twice while another root fails.
#[derive(Debug, PartialEq)]
pub(super) struct Cleanup {
    /// The retained root's remaining obligations and dependencies after it published.
    pub(super) retained: (usize, DependencyCheckpoint),
    /// Each repeated release's refusal, and whether the checkpoint stayed unchanged by all of them.
    pub(super) releases: [Option<OperatingError>; 2],
    pub(super) release_kept: bool,
    /// The failing root's two invocations.
    pub(super) failing: [Option<OperatingError>; 2],
    /// The failing and healthy probes' evaluations.
    pub(super) evaluations: (usize, usize),
    /// Whether the retained checkpoint is unchanged after the failure, and the rows its replay delivered.
    pub(super) retained_kept: bool,
    pub(super) replayed: usize,
    /// The finish's refusal.
    pub(super) finish: Option<OperatingError>,
    /// What the failing probe saw inside its evaluations.
    pub(super) seen: Cow<'static, [Seen]>,
}

/// The outer root fails inside its nested demand of the inner one; it is invoked twice.
pub(super) fn nested() -> Result<Lived, OperatingError> {
    let CompiledCase { execution, roots } = compiled();
    let probe = RuntimeProbe::new(
        Rc::clone(&execution),
        RuntimeProbeMode::NestedFailure {
            outer: roots[0],
            inner: roots[1],
        },
    );
    let mut input = TestInputs::new(&execution);
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = vec![];
    let mut context = context(&mut symbols, &mut scratch, &mut memo);
    let policy = CompletionPolicy::file(RuleIndex(0), FileId(0), &[]);
    let mut invoke = || {
        execution
            .invoke(
                execution.file_state(),
                roots[0],
                &mut input,
                &probe,
                &policy,
                &mut context,
            )
            .err()
    };
    let first = invoke();
    // Each frame was its own root's while it ran (the probe checks it inside the nested call); a
    // failed region is never evaluated again, so its frames leave the slot.
    let slots = {
        let slots = execution.file_state().slots.borrow();
        roots
            .iter()
            .map(|root| {
                let slot = &slots[root];
                let failed = if let State::Failed(error) = &slot.state {
                    Some(error.clone())
                } else {
                    None
                };
                (failed, slot.scratch.is_empty())
            })
            .collect()
    };
    let second = invoke();
    Ok(Lived::Nested {
        invocations: [first, second],
        slots,
        evaluations: probe.calls.get().0,
        seen: Cow::Owned(probe.seen.borrow().clone()),
    })
}

/// The root reenters its own region while it runs; it is invoked twice.
pub(super) fn reentered() -> Result<Lived, OperatingError> {
    let CompiledCase { execution, roots } = compiled();
    let probe = RuntimeProbe::new(
        Rc::clone(&execution),
        RuntimeProbeMode::Reenter { root: roots[0] },
    );
    let mut input = TestInputs::new(&execution);
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = vec![];
    let mut context = context(&mut symbols, &mut scratch, &mut memo);
    let policy = CompletionPolicy::file(RuleIndex(0), FileId(0), &[]);
    let mut invoke = || {
        matches!(
            execution.invoke(
                execution.file_state(),
                roots[0],
                &mut input,
                &probe,
                &policy,
                &mut context
            ),
            Ok(InvocationOutcome::Published(_))
        )
    };
    let first = invoke();
    let reentry = probe.reentry.get();
    let second = invoke();
    Ok(Lived::Reentered {
        published: [first, second],
        reentry,
        evaluations: probe.calls.get().0,
    })
}

/// The root's first replay goes to a sink that refuses it; the retry replays to one that accepts.
pub(super) fn retried() -> Result<Lived, OperatingError> {
    let CompiledCase { execution, roots } = compiled();
    let probe = RuntimeProbe::new(Rc::clone(&execution), RuntimeProbeMode::Plain);
    let mut input = TestInputs::new(&execution);
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = vec![];
    let mut context = context(&mut symbols, &mut scratch, &mut memo);
    let policy = CompletionPolicy::file(RuleIndex(0), FileId(0), &[]);
    let mut fail = Sink {
        mode: SinkMode::Reject,
        rows: vec![],
    };
    let InvocationOutcome::Published(published) = execution.invoke(
        execution.file_state(),
        roots[0],
        &mut input,
        &probe,
        &policy,
        &mut context,
    )?
    else {
        return Err(OperatingError::NotRun);
    };
    let refusal = input.replay(&published, &mut context, &mut fail).err();
    let (published_state, frames_empty) = {
        let slots = execution.file_state().slots.borrow();
        (
            matches!(slots[&roots[0]].state, State::Published(_)),
            slots[&roots[0]].scratch.is_empty(),
        )
    };
    let mut accept = Sink {
        mode: SinkMode::Accept,
        rows: vec![],
    };
    let InvocationOutcome::Published(repeated) = execution.invoke(
        execution.file_state(),
        roots[0],
        &mut input,
        &probe,
        &policy,
        &mut context,
    )?
    else {
        return Err(OperatingError::NotRun);
    };
    input.replay(&repeated, &mut context, &mut accept)?;
    Ok(Lived::Retried {
        refusal,
        published: published_state,
        frames_empty,
        replayed: accept.rows.len(),
        evaluations: probe.calls.get().0,
    })
}

/// The root runs once under this probe mode: `Plain` declines the shared walk, so its region is
/// evaluated inside its preparation poll; `Joined` waits for its cohort and is evaluated by a
/// dispatched action.
pub(super) fn counted(mode: RuntimeProbeMode) -> Result<Lived, OperatingError> {
    let CompiledCase { execution, roots } = compiled();
    let probe = RuntimeProbe::new(Rc::clone(&execution), mode);
    let mut input = TestInputs::new(&execution);
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = vec![];
    let mut context = context(&mut symbols, &mut scratch, &mut memo);
    let policy = CompletionPolicy::file(RuleIndex(0), FileId(0), &[]);
    let published = matches!(
        execution.invoke(
            execution.file_state(),
            roots[0],
            &mut input,
            &probe,
            &policy,
            &mut context
        ),
        Ok(InvocationOutcome::Published(_))
    );
    Ok(Lived::Counted {
        published,
        evaluations: probe.calls.get().0,
        counts: execution.readiness_counts(),
    })
}

/// One activated root's slot is touched 64 times, then another root's once.
pub(super) fn initialized() -> Result<Lived, OperatingError> {
    let CompiledCase { execution, roots } = compiled();
    execution.activate_file_roots(&roots, Default::default())?;
    let state = execution.file_state();
    let before = state.reader_initializations.get().0;
    let mut slots = state.slots.borrow_mut();
    for _ in 0..64 {
        Slot::touched(state, &mut slots, &execution.program, roots[0])?;
    }
    let first = state.reader_initializations.get().0 - before;
    Slot::touched(state, &mut slots, &execution.program, roots[1])?;
    Ok(Lived::Initialized {
        first,
        second: state.reader_initializations.get().0 - before,
        slots: slots.len(),
    })
}

/// Root 1 publishes for two external obligations, is released twice, and root 0 then fails with a
/// semantic error while its probe repeats the release.
pub(super) fn cleanup() -> Result<Lived, OperatingError> {
    let CompiledCase { execution, roots } = compiled();
    execution.activate_file_roots(&[roots[0], roots[1], roots[1]], Default::default())?;
    let healthy = RuntimeProbe::new(Rc::clone(&execution), RuntimeProbeMode::Plain);
    let mut input = TestInputs::new(&execution);
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = vec![];
    let mut context = context(&mut symbols, &mut scratch, &mut memo);
    let policy = CompletionPolicy::file(RuleIndex(0), FileId(0), &[]);
    let InvocationOutcome::Published(_) = execution.invoke(
        execution.file_state(),
        roots[1],
        &mut input,
        &healthy,
        &policy,
        &mut context,
    )?
    else {
        return Err(OperatingError::NotRun);
    };
    let retained = execution.retry_checkpoint(execution.file_state(), roots[1])?;
    // Negative control: preflight refuses repeated retirement atomically. The
    // kernel probe below repeats the same actual release while the new primary
    // semantic error occurs, so both errors reach production terminal handling.
    let mut releases = [None, None];
    let mut release_kept = true;
    for release in &mut releases {
        *release = execution.retire(execution.file_state(), roots[1]).err();
        release_kept &= execution.retry_checkpoint(execution.file_state(), roots[1])? == retained;
    }
    let failing = RuntimeProbe::new(
        Rc::clone(&execution),
        RuntimeProbeMode::SemanticAndCleanupFailure {
            completed: roots[1],
        },
    );
    // Both entries still traverse the actual batch scheduler. The already
    // published alias remains terminal; the other root keeps its semantic error.
    execution.prepare_roots(
        execution.file_state(),
        &[roots[0], roots[1], roots[1]],
        &mut input,
        &failing,
        &policy,
        &mut context,
    )?;
    // The primary semantic failure must not become the earlier cleanup NotRun.
    let mut failed = [None, None];
    for failure in &mut failed {
        *failure = execution
            .invoke(
                execution.file_state(),
                roots[0],
                &mut input,
                &failing,
                &policy,
                &mut context,
            )
            .err();
    }
    let evaluations = (failing.calls.get().0, healthy.calls.get().0);
    let seen = Cow::Owned(failing.seen.borrow().clone());
    let retained_kept = execution.retry_checkpoint(execution.file_state(), roots[1])? == retained;
    let InvocationOutcome::Published(publication) = execution.invoke(
        execution.file_state(),
        roots[1],
        &mut input,
        &healthy,
        &policy,
        &mut context,
    )?
    else {
        return Err(OperatingError::NotRun);
    };
    let mut sink = Sink {
        mode: SinkMode::Accept,
        rows: vec![],
    };
    input.replay(&publication, &mut context, &mut sink)?;
    drop(publication);
    drop(input);
    drop(healthy);
    drop(failing);
    let execution = Rc::try_unwrap(execution).map_err(|_| OperatingError::WrongOperation)?;
    Ok(Lived::Cleanup(Cleanup {
        retained: (retained.remaining.0, retained.dependencies),
        releases,
        release_kept,
        failing: failed,
        evaluations,
        retained_kept,
        replayed: sink.rows.len(),
        finish: execution.finish().err(),
        seen,
    }))
}
