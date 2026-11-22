//! The `pruning` service's cases, on plans written by hand.

use ryo_relations::domain::relations::types::Relation;

use super::service::without_unused;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::rewriting::types::Planned;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as Reads;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;
use std::borrow::Cow;

/// What a row asks of the service.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scenario {
    /// A step nothing reads is dropped and the steps after it are renumbered.
    AnUnreadStepGoes,
    /// A step reads a step past the plan.
    ReadPastThePlan,
    /// A ladder of so many joins, each of the step before with itself and each beside a step nothing reads, and one of twice as many.
    Ladder(usize),
}

/// What a row answers.
#[derive(Debug, PartialEq, Eq)]
enum Answer {
    /// A number the answer is.
    Number(usize),
    /// How many steps of a ladder were kept at a depth and at twice it, beside how many steps each has: the walk marks a step the
    /// first time it reaches it and follows the inputs of a marked step no more, so it follows each kept step once.
    Counted(Cow<'static, str>),
}

type Value = Answer;
type Failure = ();
/// What a row asks of the service.
type Input = Scenario;

fn column(index: u32) -> ColumnIndex {
    ColumnIndex(index)
}

fn step(operation: Operation) -> PlanStep {
    PlanStep {
        written: Default::default(),
        operation,
        scope: StepScope::CrossFile,
    }
}

fn an_unread_step_goes() -> usize {
    let steps = vec![
        step(Operation::Select {
            from: Reads::Relation(Relation::Function),
            comparisons: Vec::new(),
            kept: Vec::new(),
        }),
        step(Operation::Project {
            from: Reads::Step(StepId(0)),
            columns: vec![column(0)],
            distinct: RepeatedRows::Dropped,
        }),
        step(Operation::Project {
            from: Reads::Step(StepId(0)),
            columns: vec![column(1)],
            distinct: RepeatedRows::Dropped,
        }),
    ];
    let Planned {
        steps: kept,
        emitted,
    } = without_unused(steps, StepId(2));
    usize::from(
        kept.len() == 2
            && emitted == StepId(1)
            && matches!(&kept[1].operation, Operation::Project { from: Reads::Step(StepId(0)), columns, distinct: RepeatedRows::Dropped } if columns == &[column(1)]),
    )
}

/// 1 when a step reading past the plan crashes rather than being dropped.
fn a_read_past_the_plan_crashes() -> usize {
    let message = crash_of(|| {
        let steps = vec![step(Operation::Project {
            from: Reads::Step(StepId(7)),
            columns: vec![column(0)],
            distinct: RepeatedRows::Dropped,
        })];
        assert!(without_unused(steps, StepId(0)).steps.is_empty());
    });
    usize::from(message.contains("index out of bounds"))
}

/// What `work` crashed with: its panic's message, or nothing when it did not crash.
fn crash_of(work: impl FnOnce()) -> String {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)).map_or_else(
        |payload| {
            payload
                .downcast_ref::<&str>()
                .map(|message| (*message).to_owned())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_default()
        },
        |()| String::new(),
    )
}

/// How many steps of a ladder of `levels` joins, each of the step before with itself and each beside a step nothing reads, are kept.
fn ladder_kept(levels: usize) -> String {
    let mut steps = vec![step(Operation::Select {
        from: Reads::Relation(Relation::Function),
        comparisons: Vec::new(),
        kept: Vec::new(),
    })];
    let mut before = StepId(0);
    for _ in 0..levels {
        steps.push(step(Operation::Project {
            from: Reads::Step(before),
            columns: vec![column(0)],
            distinct: RepeatedRows::Dropped,
        }));
        steps.push(step(Operation::Join {
            left: Reads::Step(before),
            right: Reads::Step(before),
            left_on: vec![column(0)],
            right_on: vec![column(0)],
            output: Vec::new(),
            filter: Vec::new(),
            view: Vec::new(),
            reach: None,
        }));
        before = StepId(u32::try_from(steps.len().saturating_sub(1)).unwrap_or(u32::MAX));
    }
    let written = steps.len();
    let Planned {
        steps: kept,
        emitted,
    } = without_unused(steps, before);
    format!(
        "{levels} levels, {written} steps: {} kept, the last emitted as {}",
        kept.len(),
        emitted.0
    )
}

fn subject(scenario: Input) -> Value {
    match scenario {
        Scenario::AnUnreadStepGoes => Answer::Number(an_unread_step_goes()),
        Scenario::ReadPastThePlan => Answer::Number(a_read_past_the_plan_crashes()),
        Scenario::Ladder(levels) => Answer::Counted(Cow::Owned(format!(
            "{}; {}",
            ladder_kept(levels),
            ladder_kept(levels.saturating_mul(2))
        ))),
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_step_nothing_reads_goes_and_the_steps_after_it_are_renumbered"),
        word: Word::Succeeds,
        input: || Scenario::AnUnreadStepGoes,
        expect: Expect::Value(Answer::Number(1)),
    },
    Case {
        name: CaseName("a_step_reading_past_the_plan_crashes_instead_of_being_dropped"),
        word: Word::Succeeds,
        input: || Scenario::ReadPastThePlan,
        expect: Expect::Value(Answer::Number(1)),
    },
    Case {
        name: CaseName("a_ladder_of_joins_of_a_step_with_itself_is_walked_once_a_step"),
        word: Word::Succeeds,
        input: || Scenario::Ladder(16),
        expect: Expect::Value(Answer::Counted(Cow::Borrowed(
            "16 levels, 33 steps: 17 kept, the last emitted as 16; 32 levels, 65 steps: 33 kept, the last emitted as 32",
        ))),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
