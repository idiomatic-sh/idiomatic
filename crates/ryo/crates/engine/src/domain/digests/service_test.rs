//! The `digests` service's cases: a digest is pinned, so it is the same on every build, toolchain and machine; a bound value is
//! in the digest; an equal sub-plan has one digest anywhere; and the walk digests each step once, counted at a depth and at
//! twice it.

use super::service::{digested, node_digests};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, HostedNumber, Input as PlanInput, LiteralText, Operand,
    ParameterId, ParameterName, PlanComparison, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::query_bindings::types::{
    ParameterBinding, ParameterType, PlanParameter, RuleBindings,
};
use ryo_values::domain::query_plans::types::{Emit, NodeDigest, Plan};
use std::borrow::Cow;

/// What a row digests.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// Every row of a relation, then the rows of the parameter, bound to the text `a`: the two digests, as written.
    Pinned,
    /// The digests of one plan under two values of the parameter its second step names, and under one value twice.
    BoundValue,
    /// The digest of one sub-plan at two places of two plans.
    SharedSubPlan,
    /// How many steps are digested under a ladder of joins of a step with itself, at a depth and at twice it.
    Ladder,
}

type Input = Scenario;
type Value = Cow<'static, str>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_digests_of_a_fixed_plan_are_the_written_ones"),
        word: Word::Succeeds,
        input: || Scenario::Pinned,
        expect: Expect::Value(Cow::Borrowed(
            "04219e81c516e1d1706f3de0b165318d b246db0caf0b690a2a9fe828ae2f2af3",
        )),
    },
    Case {
        name: CaseName(
            "a_bound_value_is_in_the_digest_of_the_step_that_names_it_and_of_no_step_below",
        ),
        word: Word::Succeeds,
        input: || Scenario::BoundValue,
        expect: Expect::Value(Cow::Borrowed(
            "two values: the step below equal, the step that names it differ; one value twice: equal",
        )),
    },
    Case {
        name: CaseName("an_equal_sub_plan_has_one_digest_at_any_place_of_any_plan"),
        word: Word::Succeeds,
        input: || Scenario::SharedSubPlan,
        expect: Expect::Value(Cow::Borrowed(
            "at another place: equal; over another relation: differ",
        )),
    },
    Case {
        name: CaseName(
            "node_digests_visits_each_step_once_under_a_ladder_of_ten_and_of_twenty_links",
        ),
        word: Word::Succeeds,
        input: || Scenario::Ladder,
        expect: Expect::Value(Cow::Borrowed(
            "11 of 11 steps digested, 11 digests; 21 of 21 steps digested, 21 digests",
        )),
    },
];

fn step(operation: Operation) -> PlanStep {
    PlanStep {
        operation,
        scope: StepScope::CrossFile,
    }
}

/// A plan of `steps` that declares one text parameter.
fn digested_plan(steps: Vec<PlanStep>) -> Plan {
    Plan {
        observation_inputs: Vec::new(),
        parameters: vec![PlanParameter {
            name: ParameterName("word".into()),
            kind: ParameterType::Text,
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

/// The parameter bound to a text.
fn text(value: &str) -> [ParameterBinding; 1] {
    [ParameterBinding::Text(LiteralText(value.into()))]
}

/// Every row of a relation, then the rows of the parameter among the rows of the step at `first`.
fn sub_plan(relation: Relation, first: u32) -> [PlanStep; 2] {
    [
        every_row(PlanInput::Relation(relation)),
        rows_of_the_parameter(PlanInput::Step(StepId(first))),
    ]
}

/// Whether two digests are one.
fn said(left: Option<&NodeDigest>, right: Option<&NodeDigest>) -> &'static str {
    if left == right { "equal" } else { "differ" }
}

/// The digests of the fixed plan, each as its sixteen bytes in hexadecimal.
fn pinned() -> Value {
    let plan = digested_plan(sub_plan(Relation::Function, 0).into());
    let bound = text("a");
    let written: Vec<String> = node_digests(&plan, RuleBindings(&bound))
        .0
        .iter()
        .map(|digest| digest.0.iter().map(|byte| format!("{byte:02x}")).collect())
        .collect();
    Cow::Owned(written.join(" "))
}

/// One plan digested under the values `a`, `b` and `a` again: its first step names no parameter, its second names the one.
fn bound_value() -> Value {
    let plan = digested_plan(sub_plan(Relation::Function, 0).into());
    let digests = |value: &str| node_digests(&plan, RuleBindings(&text(value))).0;
    let (first, other, again) = (digests("a"), digests("b"), digests("a"));
    Cow::Owned(format!(
        "two values: the step below {}, the step that names it {}; one value twice: {}",
        said(first.first(), other.first()),
        said(first.get(1), other.get(1)),
        if first == again { "equal" } else { "differ" },
    ))
}

/// One sub-plan as the first two steps of one plan and as the second and third of another, behind the host's rows; and beside
/// it the same steps over another relation.
fn shared_sub_plan() -> Value {
    let alone = digested_plan(sub_plan(Relation::Function, 0).into());
    let mut behind = vec![step(Operation::Hosted {
        relation: HostedNumber(0),
    })];
    behind.extend(sub_plan(Relation::Function, 1));
    let behind = digested_plan(behind);
    let other = digested_plan(sub_plan(Relation::ModuleIdentity, 0).into());
    let bound = text("a");
    let digests = |plan: &Plan| node_digests(plan, RuleBindings(&bound)).0;
    let (alone, behind, other) = (digests(&alone), digests(&behind), digests(&other));
    Cow::Owned(format!(
        "at another place: {}; over another relation: {}",
        said(alone.get(1), behind.get(2)),
        said(alone.get(1), other.get(1)),
    ))
}

/// How many steps are digested for a ladder of `links` joins, each of the step before it with itself, over one relation: a walk
/// that follows every input without keeping its answer digests two to the power of the links.
fn ladder(links: u32) -> String {
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

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(match scenario {
        Scenario::Pinned => pinned(),
        Scenario::BoundValue => bound_value(),
        Scenario::SharedSubPlan => shared_sub_plan(),
        Scenario::Ladder => Cow::Owned(format!("{}; {}", ladder(10), ladder(20))),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
