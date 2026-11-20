//! What this engine's planner makes of an expanded plan: the extremum and the nearest row of each expansion run as its
//! ordered probe, as the spellings they replace do, so the pairs the expansion writes are never formed.
use super::super::service::expanded;
use super::{as_of, group_bound};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_operators::domain::physical_plan::types::logical_program::value::LogicalProgram;
use ryo_operators::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use ryo_operators::domain::physical_plan::types::{
    LogicalNode, NodeBindings, PlanInput, PlanInputList, PlanNodeId,
};
use ryo_operators::inbound::use_cases::plan::interface::PlanUseCase as _;
use ryo_operators::inbound::use_cases::plan::use_case::PlanUseCaseImpl;
use ryo_values::domain::plan_orders::types::{GroupEnd, OrderBound};
use ryo_values::domain::plan_parts::types::Input as Source;
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::query_plans::types::Plan;
use std::borrow::Cow;

/// The plan a row expands.
type Input = fn() -> Plan;
/// How many ordered probes the planner makes of the expanded plan, and the extremum they decide by.
type Value = Cow<'static, str>;
type Failure = std::convert::Infallible;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_expanded_group_max_lowers_to_an_ordered_probe_by_the_maximum"),
        word: Word::Succeeds,
        input: || || group_bound(GroupEnd::Greatest),
        expect: Expect::Value(Cow::Borrowed("1 ordered probe: Maximum")),
    },
    Case {
        name: CaseName("the_expanded_group_min_lowers_to_an_ordered_probe_by_the_minimum"),
        word: Word::Succeeds,
        input: || || group_bound(GroupEnd::Least),
        expect: Expect::Value(Cow::Borrowed("1 ordered probe: Minimum")),
    },
    Case {
        name: CaseName("the_expanded_as_of_lowers_to_an_ordered_probe_by_the_maximum"),
        word: Word::Succeeds,
        input: || || as_of(OrderBound::Before),
        expect: Expect::Value(Cow::Borrowed("1 ordered probe: Maximum")),
    },
    Case {
        name: CaseName("the_expanded_as_of_after_lowers_to_an_ordered_probe_by_the_minimum"),
        word: Word::Succeeds,
        input: || || as_of(OrderBound::After),
        expect: Expect::Value(Cow::Borrowed("1 ordered probe: Minimum")),
    },
];

/// What this engine's planner makes of `plan`, as it prints it.
fn planned(plan: &Plan) -> String {
    let inputs: Vec<Vec<PlanInput>> = plan
        .steps
        .iter()
        .map(|step| {
            named(&inputs_of(&step.operation))
                .iter()
                .map(|input| match *input {
                    Source::Step(id) => PlanInput::Node(PlanNodeId(id.0)),
                    Source::Relation(relation) => PlanInput::Relation(relation),
                    Source::BoundObservation(_) => unreachable!("the rows read no bound input"),
                })
                .collect()
        })
        .collect();
    let mut program = LogicalProgram::new();
    let mut last = PlanNodeId(0);
    for (step, inputs) in plan.steps.iter().zip(&inputs) {
        last = program.node(LogicalNode {
            operation: &step.operation,
            inputs: PlanInputList(inputs),
            bindings: NodeBindings(&[]),
            scope: step.scope,
        });
    }
    program.emit(RuleOrdinal(0), last);
    let plan = PlanUseCaseImpl::new()
        .plan(&program)
        .expect("the expanded plan is one this engine plans");
    format!("{plan:?}")
}

fn subject(plan: Input) -> Result<Value, Failure> {
    let printed = planned(&expanded(&plan()));
    let extremum = ["Maximum", "Minimum"]
        .into_iter()
        .filter(|extremum| printed.contains(extremum))
        .collect::<Vec<_>>()
        .join(" ");
    Ok(Cow::Owned(format!(
        "{} ordered probe: {extremum}",
        printed.matches("Ordered(").count()
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
