//! Compiled program lifecycle proof with a real owner.
use super::*;
mod fixtures;
mod inputs;
use crate::domain::completion::types::completion_policy::value::CompletionPolicy;
use crate::domain::{coverage::types::RuleIndex, table_scratch::types::OperatorScratch};
use fixtures::{EvaluationCount, Reentry, RuntimeProbe, RuntimeProbeMode, Sink, SinkMode, context};
use inputs::fixtures::{CompiledCase, TestInputs, compiled};
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
    for (at, root) in roots.into_iter().enumerate() {
        let slot = &slots[&root.0];
        assert!(matches!(
            slot.state,
            State::Failed(OperatingError::WrongBinding)
        ));
        assert_eq!(
            slot.scratch[0].ordered_row,
            vec![ryo_relations::domain::relations::types::Value::Number(
                ryo_relations::domain::relations::types::Integer(100 + at as i64)
            )],
            "each nested scratch frame returns to its own root"
        );
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
        assert!(matches!(slots[&roots[0].0].state, State::Published(_)));
        assert_eq!(slots[&roots[0].0].scratch[0].ordered_row.len(), 1);
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
