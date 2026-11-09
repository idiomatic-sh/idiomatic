//! The `rules` service: free functions named by verb over the rules of a run, with no IO and no ports.

use crate::domain::workers::types::RunDecisions;

use ryo_execution::inbound::use_cases::metadata::interface::RuleIndex;
use ryo_execution::inbound::use_cases::metadata::interface::{
    Governing, Mask, RuleScope, ScopePlacement,
};
use ryo_values::domain::files::types::Glob;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::runs::types::RuleNumber;

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

/// The ids of the rules that govern the file the mask was made for.
#[must_use]
pub(crate) fn governing(metadata: &impl RunDecisions, mask: &Mask, ids: &[RuleId]) -> Vec<RuleId> {
    ids.iter()
        .enumerate()
        .filter(|(rule, _)| {
            metadata.governs(mask, RuleNumber(u32::try_from(*rule).unwrap_or(u32::MAX)))
                == Governing::Governs
        })
        .map(|(_, id)| id.clone())
        .collect()
}

use ryo_values::domain::rule_requests::types::RuleSpec;
