//! Compiled program lifecycle proof with a real owner.
use super::fixtures::{
    EvaluationCount, Reentry, RuntimeProbe, RuntimeProbeMode, Sink, SinkMode, context,
};
use super::inputs::fixtures::{CompiledCase, TestInputs, compiled};
use crate::application::file_dag::program::admission::Inputs;
use crate::application::file_dag::program::runtime::service::InvocationOutcome;
use crate::application::file_dag::program::runtime::service::Slot;
use crate::application::file_dag::program::runtime::service::State;
use crate::application::file_dag::program::runtime::types::DependencyCheckpoint;
use ryo_execution_domain::domain::completion::types::completion_policy::value::CompletionPolicy;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::{coverage::types::RuleIndex, table_scratch::types::OperatorScratch};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::FileId;
use std::rc::Rc;

#[test]
fn nested_demand_preserves_both_frames_and_exact_semantic_failure() {
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
    assert!(matches!(
        execution.invoke(
            execution.file_state(),
            roots[0],
            &mut input,
            &probe,
            &policy,
            &mut context
        ),
        Err(OperatingError::WrongBinding)
    ));
    let slots = execution.file_state().slots.borrow();
    // Each frame was its own root's while it ran (the probe asserts it inside the nested call); a failed
    // region is never evaluated again, so its frames leave the slot.
    for root in roots {
        let slot = &slots[&root];
        assert!(matches!(
            slot.state,
            State::Failed(OperatingError::WrongBinding)
        ));
        assert!(slot.scratch.is_empty(), "a failed region holds no frames");
    }
    drop(slots);
    assert!(matches!(
        execution.invoke(
            execution.file_state(),
            roots[0],
            &mut input,
            &probe,
            &policy,
            &mut context
        ),
        Err(OperatingError::WrongBinding)
    ));
    assert_eq!(
        probe.calls.get(),
        EvaluationCount(2),
        "failed roots preserve their original error without rerunning"
    );
}

#[test]
fn same_region_reentry_refuses_without_corrupting_successful_publication() {
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
    assert!(matches!(
        execution.invoke(
            execution.file_state(),
            roots[0],
            &mut input,
            &probe,
            &policy,
            &mut context
        ),
        Ok(InvocationOutcome::Published(_))
    ));
    assert_eq!(probe.reentry.get(), Reentry::Refused);
    assert!(matches!(
        execution.invoke(
            execution.file_state(),
            roots[0],
            &mut input,
            &probe,
            &policy,
            &mut context
        ),
        Ok(InvocationOutcome::Published(_))
    ));
    assert_eq!(
        probe.calls.get(),
        EvaluationCount(1),
        "a retained root runs once"
    );
}

#[test]
fn downstream_error_keeps_publication_and_retry_does_not_repeat_semantics() {
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
    let InvocationOutcome::Published(published) = execution
        .invoke(
            execution.file_state(),
            roots[0],
            &mut input,
            &probe,
            &policy,
            &mut context,
        )
        .unwrap()
    else {
        panic!("canonical completed publication")
    };
    assert!(matches!(
        input.replay(&published, &mut context, &mut fail),
        Err(OperatingError::WrongBinding)
    ));
    {
        let slots = execution.file_state().slots.borrow();
        assert!(matches!(slots[&roots[0]].state, State::Published(_)));
        assert!(
            slots[&roots[0]].scratch.is_empty(),
            "a published region holds no frames: a retry replays its publication"
        );
    }
    let mut accept = Sink {
        mode: SinkMode::Accept,
        rows: vec![],
    };
    let InvocationOutcome::Published(repeated) = execution
        .invoke(
            execution.file_state(),
            roots[0],
            &mut input,
            &probe,
            &policy,
            &mut context,
        )
        .unwrap()
    else {
        panic!("retry retains canonical publication")
    };
    input.replay(&repeated, &mut context, &mut accept).unwrap();
    assert_eq!(accept.rows.len(), 1);
    assert_eq!(
        probe.calls.get(),
        EvaluationCount(1),
        "delivery retry replays the same publication without repeating semantic work"
    );
}

#[test]
fn reached_slots_initialize_reader_obligations_once() {
    let CompiledCase { execution, roots } = compiled();
    execution.activate_file_roots(&roots).unwrap();
    let state = execution.file_state();
    let before = state.reader_initializations.get();
    let mut slots = state.slots.borrow_mut();
    for _ in 0..64 {
        Slot::touched(state, &mut slots, &execution.program, roots[0]).unwrap();
    }
    assert_eq!(state.reader_initializations.get().0 - before.0, 1);
    Slot::touched(state, &mut slots, &execution.program, roots[1]).unwrap();
    assert_eq!(state.reader_initializations.get().0 - before.0, 2);
    assert_eq!(slots.len(), 2);
}

#[test]
fn repeated_consumer_release_is_atomic_and_primary_semantic_failure_wins_cleanup() {
    let CompiledCase { execution, roots } = compiled();
    execution
        .activate_file_roots(&[roots[0], roots[1], roots[1]])
        .unwrap();
    let healthy = RuntimeProbe::new(Rc::clone(&execution), RuntimeProbeMode::Plain);
    let mut input = TestInputs::new(&execution);
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = vec![];
    let mut context = context(&mut symbols, &mut scratch, &mut memo);
    let policy = CompletionPolicy::file(RuleIndex(0), FileId(0), &[]);
    assert!(matches!(
        execution.invoke(
            execution.file_state(),
            roots[1],
            &mut input,
            &healthy,
            &policy,
            &mut context
        ),
        Ok(InvocationOutcome::Published(_))
    ));
    let retained = execution
        .retry_checkpoint(execution.file_state(), roots[1])
        .unwrap();
    assert_eq!(
        retained.remaining.0, 2,
        "one completed evaluation retains both external obligations"
    );
    assert_eq!(retained.dependencies, DependencyCheckpoint::Consumed);
    // Negative control: preflight refuses repeated retirement atomically. The
    // kernel probe below repeats the same actual release while the new primary
    // semantic error occurs, so both errors reach production terminal handling.
    for _ in 0..2 {
        assert!(matches!(
            execution.retire(execution.file_state(), roots[1]),
            Err(OperatingError::NotRun)
        ));
        assert_eq!(
            execution
                .retry_checkpoint(execution.file_state(), roots[1])
                .unwrap(),
            retained
        );
    }
    let failing = RuntimeProbe::new(
        Rc::clone(&execution),
        RuntimeProbeMode::SemanticAndCleanupFailure {
            completed: roots[1],
        },
    );
    // Both entries still traverse the actual batch scheduler. The already
    // published alias remains terminal; the other root keeps its semantic error.
    execution
        .prepare_roots(
            execution.file_state(),
            &[roots[0], roots[1], roots[1]],
            &mut input,
            &failing,
            &policy,
            &mut context,
        )
        .unwrap();
    for _ in 0..2 {
        assert!(
            matches!(
                execution.invoke(
                    execution.file_state(),
                    roots[0],
                    &mut input,
                    &failing,
                    &policy,
                    &mut context
                ),
                Err(OperatingError::WrongBinding)
            ),
            "the primary semantic failure must not become the earlier cleanup NotRun"
        );
    }
    assert_eq!(
        failing.calls.get(),
        EvaluationCount(1),
        "failed batch root is never reevaluated"
    );
    assert_eq!(healthy.calls.get(), EvaluationCount(1));
    assert_eq!(
        execution
            .retry_checkpoint(execution.file_state(), roots[1])
            .unwrap(),
        retained
    );
    let InvocationOutcome::Published(publication) = execution
        .invoke(
            execution.file_state(),
            roots[1],
            &mut input,
            &healthy,
            &policy,
            &mut context,
        )
        .unwrap()
    else {
        panic!("other root's retained publication")
    };
    let mut sink = Sink {
        mode: SinkMode::Accept,
        rows: vec![],
    };
    input.replay(&publication, &mut context, &mut sink).unwrap();
    assert_eq!(
        sink.rows.len(),
        1,
        "cleanup refusal does not erase another root's publication"
    );
    drop(publication);
    drop(input);
    drop(healthy);
    drop(failing);
    let execution = Rc::try_unwrap(execution).unwrap_or_else(|_| panic!("single production owner"));
    assert!(
        matches!(execution.finish(), Err(OperatingError::NotRun)),
        "without a primary result, finish reports the real first cleanup refusal"
    );
}
