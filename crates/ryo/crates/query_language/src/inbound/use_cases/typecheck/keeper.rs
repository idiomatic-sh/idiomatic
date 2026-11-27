//! The plan that runs judges what the checks passed: a query is lowered as the executor lowers it, and what the executor's
//! lowering refuses is refused here. The steps it counts are the steps the plan that runs has.

use crate::domain::plan_lowering::service::{lowered, mark_through};
use crate::domain::plan_lowering::types::{PackPrograms, PackRule, PlanningStage, RuleContext, RuleProgram};
use crate::domain::syntax::constants::parameter_kind_named;
use ryo_values::domain::finish_plans::expressions::types::{PlanConstant, PlanNumber};
use ryo_values::domain::finish_plans::service::inputs;
use ryo_values::domain::finish_plans::arguments::types::StepNumber;
use ryo_values::domain::finish_plans::types::{FinishPlan, RuleEmit};
use ryo_values::domain::plan_parts::types::{LiteralText, WholeNumber};
use ryo_values::domain::program::types::{ParsedModules, WrittenProgram};
use ryo_values::domain::query_bindings::types::ParameterType;
use ryo_values::domain::runs::types::RuleId;
use std::collections::BTreeMap;

const ALIAS: &str = "query";

/// A value for a declared parameter: a validation has none, and the lowering is judged with one of the declared type.
fn placeholder(kind: ParameterType) -> PlanConstant {
    match kind {
        ParameterType::Flag => PlanConstant::Text(LiteralText("true".into())),
        ParameterType::Number => PlanConstant::Number(PlanNumber::Whole(WholeNumber(0))),
        ParameterType::Text => PlanConstant::Text(LiteralText("".into())),
        ParameterType::Numbers | ParameterType::Texts | ParameterType::TextSets | ParameterType::Tables => PlanConstant::List(Vec::new()),
    }
}

/// The plan that runs for `root` over `modules`, and how many steps it has, or why the executor's lowering refuses it.
pub(super) fn lowered_plan(root: WrittenProgram, modules: ParsedModules) -> Result<(FinishPlan, usize), String> {
    let mut parameters = BTreeMap::new();
    for declared in &root.parameters {
        if let Some(kind) = parameter_kind_named(&declared.kind.0) {
            let _previous = parameters.insert(declared.parameter.0.clone(), placeholder(kind));
        }
    }
    let mut context = RuleContext::default();
    let _previous = context.parameters.insert(ALIAS.into(), parameters);
    let pack = PackPrograms {
        rules: vec![PackRule { rule: RuleId(ALIAS.into()), program: RuleProgram::Parsed(Box::new(root)) }],
        fragments: modules,
        context,
    };
    let mut plan = lowered(&pack, PlanningStage::Expanded);
    if let Some(refused) = plan.unplanned.first() {
        return Err(format!("unplanned query {}", refused.reason.0));
    }
    mark_through(&mut plan, PlanningStage::Axes);
    let Some(RuleEmit::Written(emit)) = plan.rules.iter().find(|rule| rule.rule.0.as_ref() == ALIAS).map(|rule| rule.emit.clone()) else {
        return Err("missing query emit".to_owned());
    };
    let mut seen = vec![false; plan.steps.len()];
    let mut stack = vec![emit.step.0];
    let mut count = 0;
    while let Some(at) = stack.pop() {
        if std::mem::replace(&mut seen[at], true) {
            continue;
        }
        count += 1;
        stack.extend(inputs(&plan.steps[at]).into_iter().map(|input: StepNumber| input.0));
    }
    Ok((plan, count))
}
