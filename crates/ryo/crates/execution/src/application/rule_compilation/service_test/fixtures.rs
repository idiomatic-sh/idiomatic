//! The rule and cache population helpers the rule compilation cases and their eviction group share.
use super::super::service::CompiledPlans;
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::query_tokens::types::QueryBundle;
use ryo_values::domain::rule_requests::types::{RuleParameters, RuleSpec};
use ryo_values::domain::runs::types::{RuleApplicability, RuleId};
use std::sync::PoisonError;

/// A legitimate parameter-free rule with exact query identity.
pub(super) fn rule(id: &str, text: &str) -> RuleSpec {
    RuleSpec {
        id: RuleId(id.into()),
        query: QueryBundle::from(QueryText(text.into())),
        applicability: RuleApplicability::Always,
        given: Vec::new(),
        observations: Vec::new(),
        spelled: RuleParameters("".into()),
        include: Vec::new(),
        exclude: Vec::new(),
    }
}

/// The shared cache's actual population after an operation.
pub(super) fn population(plans: &CompiledPlans) -> usize {
    plans
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .plans
        .len()
}
