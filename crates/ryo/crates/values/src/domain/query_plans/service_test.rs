//! The `query_plans` service's cases, on values alone.

use super::service::{digested, node_digests, reads_of};
use super::types::{Emit, Plan};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::plan_parts::types::ColumnIndex;
use crate::domain::plan_parts::types::HostedNumber;
use crate::domain::plan_parts::types::Input as PlanInput;
use crate::domain::plan_parts::types::StepId;
use crate::domain::plan_parts::types::StepScope;
use crate::domain::plans::types::{Operation, PlanStep};
use crate::domain::query_bindings::types::{ParameterBinding, RuleBindings};
use ryo_relations::domain::relations::types::Relation;
use std::borrow::Cow;

/// What a row of the cases asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// The read set of a plan that reads two relations, the first twice, with a step and the host's rows between them.
    RepeatedReadSet,
    /// The read set of a plan of no step.
    EmptyReadSet,
    /// The digests of one plan under two values of the parameter its second step names, and under one value twice.
    BoundValueDigests,
    /// The digest of one sub-plan at two places of two plans.
    SharedSubPlanDigests,
    /// How many steps are digested under a ladder of joins of a step with itself, at a depth and at twice it.
    LadderDigests,
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
    Case {
        name: CaseName(
            "a_bound_value_is_in_the_digest_of_the_step_that_names_it_and_of_no_step_below",
        ),
        word: Word::Succeeds,
        input: || Scenario::BoundValueDigests,
        expect: Expect::Value(Cow::Borrowed(
            "two values: the step below equal, the step that names it differ; one value twice: equal",
        )),
    },
    Case {
        name: CaseName("an_equal_sub_plan_has_one_digest_at_any_place_of_any_plan"),
        word: Word::Succeeds,
        input: || Scenario::SharedSubPlanDigests,
        expect: Expect::Value(Cow::Borrowed(
            "at another place: equal; over another relation: differ",
        )),
    },
    Case {
        name: CaseName(
            "node_digests_visits_each_step_once_under_a_ladder_of_ten_and_of_twenty_links",
        ),
        word: Word::Succeeds,
        input: || Scenario::LadderDigests,
        expect: Expect::Value(Cow::Borrowed(
            "11 of 11 steps digested, 11 digests; 21 of 21 steps digested, 21 digests",
        )),
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
        Scenario::BoundValueDigests => bound_value_digests(),
        Scenario::SharedSubPlanDigests => shared_sub_plan_digests(),
        Scenario::LadderDigests => {
            Cow::Owned(format!("{}; {}", ladder_digests(10), ladder_digests(20)))
        }
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// A plan of `steps` that declares one text parameter.
fn digested_plan(steps: Vec<PlanStep>) -> Plan {
    Plan {
        observation_inputs: Vec::new(),
        parameters: vec![crate::domain::query_bindings::types::PlanParameter {
            name: crate::domain::plan_parts::types::ParameterName("word".into()),
            kind: crate::domain::query_bindings::types::ParameterType::Text,
        }],
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
    }
}

/// A step that keeps every row of `from`.
fn every_row(from: PlanInput) -> PlanStep {
    step(Operation::Select {
        from,
        comparisons: Vec::new(),
        kept: Vec::new(),
    })
}

/// A step that keeps the rows of `from` whose first column equals the plan's parameter.
fn rows_of_the_parameter(from: PlanInput) -> PlanStep {
    use crate::domain::plan_parts::types::{
        ComparisonOrigin, Operand, ParameterId, PlanComparison, Test,
    };
    step(Operation::Select {
        from,
        comparisons: vec![PlanComparison {
            test: Test::Equal,
            left: Operand::Column(ColumnIndex(0)),
            right: Operand::Parameter(ParameterId(0)),
            origin: ComparisonOrigin::Unrecorded,
        }],
        kept: Vec::new(),
    })
}

/// Whether two digests are one.
const fn said(equal: bool) -> &'static str {
    if equal { "equal" } else { "differ" }
}

/// One plan digested under the values `a`, `b` and `a` again: its first step names no parameter, its second names the one.
fn bound_value_digests() -> Value {
    let plan = digested_plan(vec![
        every_row(PlanInput::Relation(Relation::Function)),
        rows_of_the_parameter(PlanInput::Step(StepId(0))),
    ]);
    let text = |value: &str| {
        [ParameterBinding::Text(
            crate::domain::plan_parts::types::LiteralText(value.into()),
        )]
    };
    let (first, other, again) = (text("a"), text("b"), text("a"));
    let digests = |bound: &[ParameterBinding]| node_digests(&plan, RuleBindings(bound)).0;
    let (first, other, again) = (digests(&first), digests(&other), digests(&again));
    Cow::Owned(format!(
        "two values: the step below {}, the step that names it {}; one value twice: {}",
        said(first.first() == other.first()),
        said(first.get(1) == other.get(1)),
        said(first == again),
    ))
}

/// One sub-plan (every row of a relation, then the rows of the parameter) as the first two steps of one plan and as the second
/// and third of another, behind the host's rows; and beside it the same steps over another relation.
fn shared_sub_plan_digests() -> Value {
    let sub_plan = |relation, first: u32| {
        [
            every_row(PlanInput::Relation(relation)),
            rows_of_the_parameter(PlanInput::Step(StepId(first))),
        ]
    };
    let alone = digested_plan(sub_plan(Relation::Function, 0).into());
    let mut behind = vec![step(Operation::Hosted {
        relation: HostedNumber(0),
    })];
    behind.extend(sub_plan(Relation::Function, 1));
    let behind = digested_plan(behind);
    let other = digested_plan(sub_plan(Relation::ModuleIdentity, 0).into());
    let bound = [ParameterBinding::Text(
        crate::domain::plan_parts::types::LiteralText("a".into()),
    )];
    let digests = |plan: &Plan| node_digests(plan, RuleBindings(&bound)).0;
    let (alone, behind, other) = (digests(&alone), digests(&behind), digests(&other));
    Cow::Owned(format!(
        "at another place: {}; over another relation: {}",
        said(alone.get(1) == behind.get(2)),
        said(alone.get(1) == other.get(1)),
    ))
}

/// How many steps are digested for a ladder of `links` joins, each of the step before it with itself, over one relation: a walk
/// that follows every input without keeping its answer digests two to the power of the links.
fn ladder_digests(links: u32) -> String {
    let mut steps = vec![every_row(PlanInput::Relation(Relation::Function))];
    for link in 0..links {
        steps.push(step(Operation::Join {
            left: PlanInput::Step(StepId(link)),
            right: PlanInput::Step(StepId(link)),
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
            output: Vec::new(),
            filter: Vec::new(),
            view: Vec::new(),
            reach: None,
        }));
    }
    let plan = digested_plan(steps);
    let walked = digested(&plan, RuleBindings(&[]));
    let mut distinct = walked.digests.0.clone();
    distinct.sort_unstable();
    distinct.dedup();
    format!(
        "{} of {} steps digested, {} digests",
        walked.steps.0,
        plan.steps.len(),
        distinct.len()
    )
}
