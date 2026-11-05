//! Transition controls for the production readiness queue, without a second evaluator.
use super::value::ReadyQueue;
use crate::domain::{
    executable_program::types::RegionId,
    operating::errors::OperatingError,
    readiness::types::{ReadinessAction as Action, ReadinessOutcome as Outcome, RegionCount},
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
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
fn large_unreached_program_has_only_actual_readiness_entries() {
    let mut queue = ReadyQueue::new(RegionCount(10_000_000), &[RegionId(7), RegionId(7)]).unwrap();
    assert_eq!(queue.reached_entries(), 1);
    assert_eq!(queue.next(), Ok(Some(Action::Prepare(RegionId(7)))));
    assert_eq!(
        queue.observe(RegionId(7), Outcome::Waiting(RegionId(10_000_000))),
        Err(OperatingError::NotRun)
    );
    assert_eq!(
        queue.reached_entries(),
        1,
        "invalid dependency cannot allocate a readiness entry"
    );
    queue
        .observe(RegionId(7), Outcome::Waiting(RegionId(9_999_999)))
        .unwrap();
    assert_eq!(queue.reached_entries(), 2);
    assert_eq!(queue.next(), Ok(Some(Action::Prepare(RegionId(9_999_999)))));
    queue
        .observe(RegionId(9_999_999), Outcome::Terminal)
        .unwrap();
    assert_eq!(queue.next(), Ok(Some(Action::Prepare(RegionId(7)))));
    queue.observe(RegionId(7), Outcome::Terminal).unwrap();
    assert_eq!(queue.next(), Ok(None));
    assert_eq!(queue.reached_entries(), 2);
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
];
fn subject(case: Input) -> Result<Value, Failure> {
    case();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
