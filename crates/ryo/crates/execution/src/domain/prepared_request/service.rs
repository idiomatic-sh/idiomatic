//! Resolve caller plans against actual declarations before executable preparation.
use super::types::Compiled;
use crate::domain::file_program::{
    errors::ObservationError,
    service::{link, observations},
};
use ryo_values::domain::{
    plans::types::Plan, query_bindings::types::ParameterBinding, rule_requests::types::RuleSpec,
};
pub(crate) fn assemble(
    plans: Vec<Plan>,
    bindings: Vec<Vec<ParameterBinding>>,
    rules: &[RuleSpec],
) -> Result<Compiled, ObservationError> {
    if plans.len() != rules.len() || bindings.len() != rules.len() {
        return Err(ObservationError::InvalidReference);
    }
    let schedule = observations::schedule(rules)?;
    let borrowed = plans.iter().map(Some).collect::<Vec<_>>();
    let resolved = rules
        .iter()
        .map(|rule| observations::schemas(rule, rules, &borrowed).map(|prepared| prepared.bindings))
        .collect::<Result<Vec<_>, _>>()?;
    let linked = link::link(plans, resolved)?;
    Ok(Compiled {
        plans: linked.plans,
        bindings,
        schedule,
        observations: linked.bindings,
    })
}
