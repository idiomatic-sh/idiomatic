//! The `rules` service: free functions named by verb over the rules of a run, with no IO and no ports.

use super::constants::PLACE_OF_RULE;
use super::errors::RulesError;
use super::types::RuleSpec;
use crate::domain::coverage::types::RuleIndex;
use crate::domain::finish::types::RuleNumber;
use crate::domain::placement::service::governs;
use crate::domain::placement::types::{Governing, Mask, RuleScope, ScopePlacement};
use crate::domain::table_model::types::ParameterBinding;
use ryo_values::domain::files::types::Glob;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::runs::types::RuleId;

/// Refuses a rule with no id, or with the id of an earlier one.
///
/// # Errors
///
/// `BlankRuleId` for an id with nothing in it; `DuplicateRuleId` for an id an earlier rule has.
pub(crate) fn check_ids(rules: &[RuleSpec]) -> Result<(), RulesError> {
    for (place, rule) in rules.iter().enumerate() {
        if rule.id.0.trim().is_empty() {
            return Err(RulesError::BlankRuleId);
        }
        if rules
            .iter()
            .take(place)
            .any(|earlier| earlier.id == rule.id)
        {
            return Err(RulesError::DuplicateRuleId {
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
pub(crate) fn bind(plan: &Plan, rule: &RuleSpec) -> Result<Vec<ParameterBinding>, RulesError> {
    let mut bound: Vec<ParameterBinding> = Vec::new();
    for parameter in &plan.parameters {
        let given = rule.given.iter().find(|given| given.name == parameter.name);
        bound.push(
            given
                .ok_or_else(|| RulesError::MissingParameter {
                    id: rule.id.clone(),
                })?
                .value
                .clone(),
        );
    }
    Ok(bound)
}

/// The ids of the rules, in the order of the rules.
#[must_use]
pub(crate) fn ids_of(rules: &[RuleSpec]) -> Vec<RuleId> {
    rules.iter().map(|rule| rule.id.clone()).collect()
}

/// The globs of every rule, as the scopes placement decides on.
#[must_use]
pub(crate) fn scopes_of(rules: &[RuleSpec]) -> Vec<RuleScope> {
    rules
        .iter()
        .map(|rule| RuleScope {
            include: rule.include.clone(),
            exclude: rule.exclude.clone(),
        })
        .collect()
}

/// The id of the rule an invalid glob belongs to, when the placement knows whose glob it is.
#[must_use]
pub(crate) fn owner_of<'a>(
    placement: &ScopePlacement,
    glob: &Glob,
    ids: &'a [RuleId],
) -> Option<&'a RuleId> {
    let place = placement.globs.iter().position(|each| each.0 == glob.0)?;
    let owner = placement.owners.get(place)?.first()?;
    ids.get(usize::try_from(owner.rule.0).unwrap_or(usize::MAX))
}

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

/// The ids of the rules that govern the file the mask was made for.
#[must_use]
pub(crate) fn governing(mask: &Mask, ids: &[RuleId]) -> Vec<RuleId> {
    ids.iter()
        .enumerate()
        .filter(|(rule, _)| {
            governs(mask, RuleNumber(u32::try_from(*rule).unwrap_or(u32::MAX)))
                == Governing::Governs
        })
        .map(|(_, id)| id.clone())
        .collect()
}

/// Why a rule failed, in the words of what refused it.
#[must_use]
pub(crate) fn failed(id: &RuleId, reason: &dyn std::fmt::Display) -> RulesError {
    RulesError::Failed {
        id: id.clone(),
        reason: reason.to_string(),
    }
}
