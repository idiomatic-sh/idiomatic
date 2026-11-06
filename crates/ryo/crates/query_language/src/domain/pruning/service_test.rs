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

/// What a row asks of the service.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scenario {
    /// A step nothing reads is dropped and the steps after it are renumbered.
    AnUnreadStepGoes,
    /// A step reads a step past the plan.
    ReadPastThePlan,
}

/// A number the answer is.
type Value = usize;
type Failure = ();
/// What a row asks of the service.
type Input = Scenario;

fn column(index: u32) -> ColumnIndex {
    ColumnIndex(index)
}

fn step(operation: Operation) -> PlanStep {
    PlanStep {
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

fn subject(scenario: Input) -> Value {
    match scenario {
        Scenario::AnUnreadStepGoes => an_unread_step_goes(),
        Scenario::ReadPastThePlan => a_read_past_the_plan_crashes(),
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_step_nothing_reads_goes_and_the_steps_after_it_are_renumbered"),
        word: Word::Succeeds,
        input: || Scenario::AnUnreadStepGoes,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_step_reading_past_the_plan_crashes_instead_of_being_dropped"),
        word: Word::Succeeds,
        input: || Scenario::ReadPastThePlan,
        expect: Expect::Value(1),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
