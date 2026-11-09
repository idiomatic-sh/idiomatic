//! Transition controls for the production readiness queue, without a second evaluator.
use super::value::ReadyQueue;
use crate::domain::{
    executable_program::types::RegionId,
    operating::errors::OperatingError,
    readiness::types::{
        ReadinessAction as Action, ReadinessEntryCount, ReadinessOutcome as Outcome, RegionCount,
    },
};
fn diamond_preserves_cohorts_and_wakes_each_actual_waiter_once() {
    let mut queue =
        ReadyQueue::new(RegionCount(4), &[RegionId(0), RegionId(1), RegionId(0)]).unwrap();
    let script = [
        (Action::Prepare(RegionId(0)), Outcome::Waiting(RegionId(2))),
        (Action::Prepare(RegionId(1)), Outcome::Waiting(RegionId(2))),
        (Action::Prepare(RegionId(2)), Outcome::Waiting(RegionId(3))),
        (Action::Prepare(RegionId(3)), Outcome::Ready),
        (Action::Evaluate(RegionId(3)), Outcome::Terminal),
        (Action::Prepare(RegionId(2)), Outcome::Ready),
        (Action::Evaluate(RegionId(2)), Outcome::Terminal),
        (Action::Prepare(RegionId(0)), Outcome::Ready),
        (Action::Prepare(RegionId(1)), Outcome::Ready),
        (Action::Evaluate(RegionId(0)), Outcome::Terminal),
        (Action::Evaluate(RegionId(1)), Outcome::Terminal),
    ];
    for (action, outcome) in script {
        assert_eq!(queue.next(), Ok(Some(action)));
        let id = match action {
            Action::Prepare(id) | Action::Evaluate(id) => id,
        };
        queue.observe(id, outcome).unwrap();
    }
    assert_eq!(queue.next(), Ok(None));
    let counts = queue.counts();
    assert_eq!(counts.preparations.0, 7);
    assert_eq!(counts.evaluations.0, 4);
    assert_eq!(counts.suspensions.0, 3);
    assert_eq!(counts.wakeups.0, 3);
}
fn deep_chain_has_linear_admission_work() {
    let depth = 2048;
    let mut queue = ReadyQueue::new(RegionCount(depth), &[RegionId(0)]).unwrap();
    for id in 0..depth {
        assert_eq!(queue.next(), Ok(Some(Action::Prepare(RegionId(id)))));
        queue
            .observe(
                RegionId(id),
                if id + 1 == depth {
                    Outcome::Ready
                } else {
                    Outcome::Waiting(RegionId(id + 1))
                },
            )
            .unwrap();
    }
    for id in (0..depth).rev() {
        if id + 1 != depth {
            assert_eq!(queue.next(), Ok(Some(Action::Prepare(RegionId(id)))));
            queue.observe(RegionId(id), Outcome::Ready).unwrap();
        }
        assert_eq!(queue.next(), Ok(Some(Action::Evaluate(RegionId(id)))));
        queue.observe(RegionId(id), Outcome::Terminal).unwrap();
    }
    assert_eq!(queue.next(), Ok(None));
    assert_eq!(queue.counts().preparations.0, (2 * depth - 1) as u64);
    assert_eq!(queue.counts().suspensions.0, (depth - 1) as u64);
    assert_eq!(queue.counts().wakeups.0, (depth - 1) as u64);
}
fn terminal_failure_wakes_parents_but_ready_alone_does_not() {
    let mut queue = ReadyQueue::new(RegionCount(3), &[RegionId(0), RegionId(1)]).unwrap();
    for id in [0, 1] {
        assert_eq!(queue.next(), Ok(Some(Action::Prepare(RegionId(id)))));
        queue
            .observe(RegionId(id), Outcome::Waiting(RegionId(2)))
            .unwrap();
    }
    assert_eq!(queue.next(), Ok(Some(Action::Prepare(RegionId(2)))));
    queue.observe(RegionId(2), Outcome::Ready).unwrap();
    assert_eq!(queue.next(), Ok(Some(Action::Evaluate(RegionId(2)))));
    // A failed actual poll and a published actual poll both produce Terminal.
    queue.observe(RegionId(2), Outcome::Terminal).unwrap();
    for id in [0, 1] {
        assert_eq!(queue.next(), Ok(Some(Action::Prepare(RegionId(id)))));
        queue.observe(RegionId(id), Outcome::Terminal).unwrap();
    }
    assert_eq!(queue.next(), Ok(None));
    assert_eq!(queue.counts().wakeups.0, 2);
}
fn undiscovered_nodes_are_never_admitted_and_cycles_refuse() {
    let mut queue = ReadyQueue::new(RegionCount(3), &[RegionId(0)]).unwrap();
    assert_eq!(queue.next(), Ok(Some(Action::Prepare(RegionId(0)))));
    queue.observe(RegionId(0), Outcome::Ready).unwrap();
    assert_eq!(queue.next(), Ok(Some(Action::Evaluate(RegionId(0)))));
    queue.observe(RegionId(0), Outcome::Terminal).unwrap();
    assert_eq!(queue.next(), Ok(None));
    assert_eq!(queue.counts().preparations.0, 1);
    let mut cycle = ReadyQueue::new(RegionCount(2), &[RegionId(0)]).unwrap();
    assert_eq!(cycle.next(), Ok(Some(Action::Prepare(RegionId(0)))));
    cycle
        .observe(RegionId(0), Outcome::Waiting(RegionId(1)))
        .unwrap();
    assert_eq!(cycle.next(), Ok(Some(Action::Prepare(RegionId(1)))));
    cycle
        .observe(RegionId(1), Outcome::Waiting(RegionId(0)))
        .unwrap();
    assert_eq!(cycle.next(), Err(OperatingError::NotRun));
}
fn a_completed_producer_cannot_be_waited_on_again() {
    let mut queue = ReadyQueue::new(RegionCount(2), &[RegionId(0), RegionId(1)]).unwrap();
    assert_eq!(queue.next(), Ok(Some(Action::Prepare(RegionId(0)))));
    queue.observe(RegionId(0), Outcome::Terminal).unwrap();
    assert_eq!(queue.next(), Ok(Some(Action::Prepare(RegionId(1)))));
    assert_eq!(
        queue.observe(RegionId(1), Outcome::Waiting(RegionId(0))),
        Err(OperatingError::NotRun)
    );
}
fn preparation_terminal_wakes_admissions_before_the_ready_cohort() {
    let mut queue = ReadyQueue::new(RegionCount(3), &[RegionId(0), RegionId(1)]).unwrap();
    assert_eq!(queue.next(), Ok(Some(Action::Prepare(RegionId(0)))));
    queue
        .observe(RegionId(0), Outcome::Waiting(RegionId(2)))
        .unwrap();
    assert_eq!(queue.next(), Ok(Some(Action::Prepare(RegionId(1)))));
    queue.observe(RegionId(1), Outcome::Ready).unwrap();
    assert_eq!(queue.next(), Ok(Some(Action::Prepare(RegionId(2)))));
    queue.observe(RegionId(2), Outcome::Terminal).unwrap();
    assert_eq!(queue.next(), Ok(Some(Action::Prepare(RegionId(0)))));
    queue.observe(RegionId(0), Outcome::Ready).unwrap();
    assert_eq!(queue.next(), Ok(Some(Action::Evaluate(RegionId(1)))));
    queue.observe(RegionId(1), Outcome::Terminal).unwrap();
    assert_eq!(queue.next(), Ok(Some(Action::Evaluate(RegionId(0)))));
    queue.observe(RegionId(0), Outcome::Terminal).unwrap();
    assert_eq!(queue.next(), Ok(None));
}
/// Runs `script` to the end: each step is the action the queue must dispatch, whether its poll evaluated the region in
/// preparation, and the outcome the poll answers.
fn run_script(queue: &mut ReadyQueue, script: &[(Action, bool, Outcome)]) {
    for &(action, inline, outcome) in script {
        assert_eq!(queue.next(), Ok(Some(action)));
        let id = match action {
            Action::Prepare(id) | Action::Evaluate(id) => id,
        };
        if inline {
            queue.evaluated_in_preparation(id).unwrap();
        }
        queue.observe(id, outcome).unwrap();
    }
    assert_eq!(queue.next(), Ok(None));
}
fn an_evaluation_in_preparation_is_counted_once_by_the_queue() {
    // A region that joins no shared walk is evaluated inside its preparation poll and suspends on nested demand; its
    // resumed evaluation is dispatched. The counts equal those of the same region evaluated only by dispatch.
    let mut inline = ReadyQueue::new(RegionCount(2), &[RegionId(0)]).unwrap();
    run_script(
        &mut inline,
        &[
            (
                Action::Prepare(RegionId(0)),
                true,
                Outcome::Waiting(RegionId(1)),
            ),
            (Action::Prepare(RegionId(1)), true, Outcome::Terminal),
            (Action::Prepare(RegionId(0)), false, Outcome::Ready),
            (Action::Evaluate(RegionId(0)), false, Outcome::Terminal),
        ],
    );
    let mut dispatched = ReadyQueue::new(RegionCount(2), &[RegionId(0)]).unwrap();
    run_script(
        &mut dispatched,
        &[
            (Action::Prepare(RegionId(0)), false, Outcome::Ready),
            (
                Action::Evaluate(RegionId(0)),
                false,
                Outcome::Waiting(RegionId(1)),
            ),
            (Action::Prepare(RegionId(1)), false, Outcome::Ready),
            (Action::Evaluate(RegionId(1)), false, Outcome::Terminal),
            (Action::Prepare(RegionId(0)), false, Outcome::Ready),
            (Action::Evaluate(RegionId(0)), false, Outcome::Terminal),
        ],
    );
    assert_eq!(inline.counts().evaluations.0, 3);
    assert_eq!(inline.counts().suspensions.0, 1);
    assert_eq!(inline.counts().wakeups.0, 1);
    assert_eq!(inline.counts().preparations.0, 3);
    assert_eq!(dispatched.counts().evaluations, inline.counts().evaluations);
    assert_eq!(dispatched.counts().suspensions, inline.counts().suspensions);
    assert_eq!(dispatched.counts().wakeups, inline.counts().wakeups);
}
fn an_evaluation_in_preparation_is_recorded_only_for_a_preparation_poll() {
    let mut queue = ReadyQueue::new(RegionCount(1), &[RegionId(0)]).unwrap();
    // Not polling: nothing to record.
    assert_eq!(
        queue.evaluated_in_preparation(RegionId(0)),
        Err(OperatingError::NotRun)
    );
    assert_eq!(queue.next(), Ok(Some(Action::Prepare(RegionId(0)))));
    queue.evaluated_in_preparation(RegionId(0)).unwrap();
    // The poll is an evaluation now: a second record would count it twice.
    assert_eq!(
        queue.evaluated_in_preparation(RegionId(0)),
        Err(OperatingError::NotRun)
    );
    queue.observe(RegionId(0), Outcome::Ready).unwrap();
    assert_eq!(queue.next(), Ok(Some(Action::Evaluate(RegionId(0)))));
    // A dispatched evaluation is already counted.
    assert_eq!(
        queue.evaluated_in_preparation(RegionId(0)),
        Err(OperatingError::NotRun)
    );
    queue.observe(RegionId(0), Outcome::Terminal).unwrap();
    assert_eq!(queue.counts().evaluations.0, 2);
}
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
fn large_unreached_program_has_only_actual_readiness_entries() {
    let mut queue = ReadyQueue::new(RegionCount(10_000_000), &[RegionId(7), RegionId(7)]).unwrap();
    assert_eq!(queue.reached_entries(), ReadinessEntryCount(1));
    assert_eq!(queue.next(), Ok(Some(Action::Prepare(RegionId(7)))));
    assert_eq!(
        queue.observe(RegionId(7), Outcome::Waiting(RegionId(10_000_000))),
        Err(OperatingError::NotRun)
    );
    assert_eq!(
        queue.reached_entries(),
        ReadinessEntryCount(1),
        "invalid dependency cannot allocate a readiness entry"
    );
    queue
        .observe(RegionId(7), Outcome::Waiting(RegionId(9_999_999)))
        .unwrap();
    assert_eq!(queue.reached_entries(), ReadinessEntryCount(2));
    assert_eq!(queue.next(), Ok(Some(Action::Prepare(RegionId(9_999_999)))));
    queue
        .observe(RegionId(9_999_999), Outcome::Terminal)
        .unwrap();
    assert_eq!(queue.next(), Ok(Some(Action::Prepare(RegionId(7)))));
    queue.observe(RegionId(7), Outcome::Terminal).unwrap();
    assert_eq!(queue.next(), Ok(None));
    assert_eq!(queue.reached_entries(), ReadinessEntryCount(2));
}
type Input = fn();
type Value = ();
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("large unreached program allocates only reached readiness entries"),
        word: Word::Succeeds,
        input: || large_unreached_program_has_only_actual_readiness_entries,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("preparation terminal wakes before ready evaluation"),
        word: Word::Succeeds,
        input: || preparation_terminal_wakes_admissions_before_the_ready_cohort,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("waiting on a terminal producer refuses instead of spinning"),
        word: Word::Succeeds,
        input: || a_completed_producer_cannot_be_waited_on_again,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("diamond and duplicate roots preserve cohort order"),
        word: Word::Succeeds,
        input: || diamond_preserves_cohorts_and_wakes_each_actual_waiter_once,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("deep chain has linear admission work"),
        word: Word::Succeeds,
        input: || deep_chain_has_linear_admission_work,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("terminal failure wakes exact waiters"),
        word: Word::Succeeds,
        input: || terminal_failure_wakes_parents_but_ready_alone_does_not,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("unreached nodes stay unpolled and cycles refuse"),
        word: Word::Succeeds,
        input: || undiscovered_nodes_are_never_admitted_and_cycles_refuse,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("an evaluation in preparation is counted once by the queue"),
        word: Word::Succeeds,
        input: || an_evaluation_in_preparation_is_counted_once_by_the_queue,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("an evaluation in preparation is recorded only for a preparation poll"),
        word: Word::Succeeds,
        input: || an_evaluation_in_preparation_is_recorded_only_for_a_preparation_poll,
        expect: Expect::Value(()),
    },
];
fn subject(case: Input) -> Result<Value, Failure> {
    case();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
