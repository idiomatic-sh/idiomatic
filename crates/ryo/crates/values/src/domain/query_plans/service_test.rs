//! The `query_plans` service's cases, on values alone.

use super::service::reads_of;
use super::types::{Emit, Plan};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::plan_parts::types::ColumnIndex;
use crate::domain::plan_parts::types::HostedNumber;
use crate::domain::plan_parts::types::Input as PlanInput;
use crate::domain::plan_parts::types::StepId;
use crate::domain::plan_parts::types::StepScope;
use crate::domain::plans::types::{Operation, PlanStep};
use ryo_relations::domain::relations::types::Relation;
use std::borrow::Cow;

mod digests;
mod unfolding;

/// What a row of the cases asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// The read set of a plan that reads two relations, the first twice, with a step and the host's rows between them.
    RepeatedReadSet,
    /// The read set of a plan of no step.
    EmptyReadSet,
}

/// What a row of the cases asks of the service.
type Input = Scenario;
/// The read set, as text.
type Value = Cow<'static, str>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_plans_reads_are_each_relation_once_in_first_read_order"),
        word: Word::Succeeds,
        input: || Scenario::RepeatedReadSet,
        expect: Expect::Value(Cow::Borrowed("[ModuleIdentity, Function]")),
    },
    Case {
        name: CaseName("a_plan_of_no_step_reads_nothing"),
        word: Word::Succeeds,
        input: || Scenario::EmptyReadSet,
        expect: Expect::Value(Cow::Borrowed("[]")),
    },
];

/// A step of `operation`, after the files.
fn step(operation: Operation) -> PlanStep {
    PlanStep {
        operation,
        scope: StepScope::CrossFile,
    }
}

/// The read set of a plan of `steps`, as text.
fn read_set(steps: Vec<PlanStep>) -> Value {
    let plan = Plan {
        observation_inputs: Vec::new(),
        parameters: Vec::new(),
        steps,
        comparison_origins: Vec::new(),
        observations: Vec::new(),
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        frontier: Vec::new(),
    };
    Cow::Owned(format!("{:?}", reads_of(&plan).0))
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let relation = PlanInput::Relation(Relation::Function);
    let select = |from| Operation::Select {
        from,
        comparisons: Vec::new(),
        kept: Vec::new(),
    };
    Ok(match scenario {
        Scenario::RepeatedReadSet => read_set(vec![
            step(select(PlanInput::Relation(Relation::ModuleIdentity))),
            step(Operation::Hosted {
                relation: HostedNumber(0),
            }),
            step(Operation::Join {
                left: PlanInput::Step(StepId(0)),
                right: relation,
                left_on: Vec::new(),
                right_on: Vec::new(),
                output: Vec::new(),
                filter: Vec::new(),
                view: Vec::new(),
                reach: None,
            }),
            step(select(PlanInput::Relation(Relation::ModuleIdentity))),
        ]),
        Scenario::EmptyReadSet => read_set(Vec::new()),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
