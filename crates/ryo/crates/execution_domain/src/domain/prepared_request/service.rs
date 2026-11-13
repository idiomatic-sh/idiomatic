//! Resolve caller plans against actual declarations before executable preparation.
use super::types::Compiled;
use crate::domain::observation_schedule::service::link::service::link;
use crate::domain::observation_schedule::service::{schedule, schemas};
use crate::domain::rule_intent::errors::RuleIntentError;
use ryo_values::domain::{
    plans::types::Plan, query_bindings::types::ParameterBinding, rule_requests::types::RuleSpec,
};
/// Assembles the compiled request from its plans, parameter bindings and rules, scheduling their observations.
///
/// # Errors
///
/// [`RuleIntentError::InvalidReference`] when the plans, bindings and rules differ in number, and the error scheduling,
/// resolving or linking their observations refuses them with.
pub fn assemble(
    plans: Vec<Plan>,
    bindings: Vec<Vec<ParameterBinding>>,
    rules: &[RuleSpec],
) -> Result<Compiled, RuleIntentError> {
    if plans.len() != rules.len() || bindings.len() != rules.len() {
        return Err(RuleIntentError::InvalidReference);
    }
    let schedule = schedule(rules)?;
    let borrowed = plans.iter().map(Some).collect::<Vec<_>>();
    let resolved = rules
        .iter()
        .map(|rule| schemas(rule, rules, &borrowed).map(|prepared| prepared.bindings))
        .collect::<Result<Vec<_>, _>>()?;
    let linked = link(plans, resolved)?;
    Ok(Compiled {
        plans: linked.plans,
        bindings,
        schedule,
        observations: linked.bindings,
    })
}
