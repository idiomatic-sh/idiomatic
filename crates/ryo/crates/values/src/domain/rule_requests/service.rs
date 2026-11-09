//! The `rule_requests` service: free functions named by verb over the rules of a run, with no IO and no ports.

use super::errors::RuleRequestsError;
use super::types::RuleSpec;
use crate::domain::plans::types::Plan;
use crate::domain::query_bindings::types::ParameterBinding;
use crate::domain::runs::types::RuleId;

/// Refuses a rule with no id, or with the id of an earlier one.
///
/// # Errors
///
/// `BlankRuleId` for an id with nothing in it; `DuplicateRuleId` for an id an earlier rule has.
pub fn check_ids(rules: &[RuleSpec]) -> Result<(), RuleRequestsError> {
    for (place, rule) in rules.iter().enumerate() {
        if rule.id.0.trim().is_empty() {
            return Err(RuleRequestsError::BlankRuleId);
        }
        if rules
            .iter()
            .take(place)
            .any(|earlier| earlier.id == rule.id)
        {
            return Err(RuleRequestsError::DuplicateRuleId {
                id: rule.id.clone(),
            });
        }
    }
    Ok(())
}

/// What the rule binds the parameters of its plan to: the rule gives them by name, the plan reads them by place.
///
/// # Errors
///
/// `MissingParameter` when the plan declares a parameter the rule gives nothing for.
pub fn bind(plan: &Plan, rule: &RuleSpec) -> Result<Vec<ParameterBinding>, RuleRequestsError> {
    let mut bound: Vec<ParameterBinding> = Vec::new();
    for parameter in &plan.parameters {
        let given = rule.given.iter().find(|given| given.name == parameter.name);
        bound.push(
            given
                .ok_or_else(|| RuleRequestsError::MissingParameter {
                    id: rule.id.clone(),
                })?
                .value
                .clone(),
        );
    }
    Ok(bound)
}

/// Why a rule failed, in the words of what refused it.
#[must_use]
pub fn failed(id: &RuleId, reason: &dyn std::fmt::Display) -> RuleRequestsError {
    RuleRequestsError::Failed {
        id: id.clone(),
        reason: reason.to_string(),
    }
}
