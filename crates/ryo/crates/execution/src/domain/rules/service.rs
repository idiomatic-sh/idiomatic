//! Execution attribution reads the request-owned exact rule identity.
use super::constants::PLACE_OF_RULE;
use crate::domain::coverage::types::RuleIndex;
use ryo_values::domain::runs::types::RuleId;
/// The id of the rule at `place`.
///
/// # Panics
///
/// On a place past the rules: the places come only from the rules of the run, so one past them is a bug.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "a rule's place comes only from the rules of the run; one past them is a bug that must crash, not a blank id that hides the rule"
)]
pub(crate) fn id_at(ids: &[RuleId], place: RuleIndex) -> RuleId {
    ids.get(place.0).cloned().expect(PLACE_OF_RULE)
}

use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::rule_requests::{errors::RuleRequestsError, types::RuleSpec};
/// Refuses a rule with no id, or with the id of an earlier one.
///
/// # Errors
///
/// `BlankRuleId` for an id with nothing in it; `DuplicateRuleId` for an id an earlier rule has.
pub(crate) fn check_ids(rules: &[RuleSpec]) -> Result<(), RuleRequestsError> {
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
pub(crate) fn bind(plan: &Plan, rule: &RuleSpec) -> Result<Vec<ParameterBinding>, RuleRequestsError> {
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
