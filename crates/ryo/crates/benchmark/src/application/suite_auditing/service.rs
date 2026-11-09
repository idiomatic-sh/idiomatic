//! Audit decoded authored facts through the existing domain inventory policies.
use crate::domain::suite::service as policy;
use crate::outbound::suite::{interface::SuitePort, types::ArtifactPath};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::{reports::types::AuthoredInventory, workloads::types::Tool};
use std::collections::BTreeSet;
use std::path::Path;
/// Observe authored definitions in filesystem order and validate their semantic inventory.
pub(crate) fn audit(
    suite: &dyn SuitePort,
    directory: &Path,
    tool: &Tool,
) -> Result<crate::domain::suite::types::audit::value::Audit, Error> {
    suite.verify_query_modules(ArtifactPath(directory), tool)?;
    let defaults: BTreeSet<_> = tool.defaults.iter().cloned().collect();
    if defaults.is_empty() || defaults.len() != tool.defaults.len() {
        return Err(Error::Invalid(super::constants::INVALID_DEFAULTS.into()));
    }
    for (rule, issue) in tool.deferred.iter().chain(&tool.pending) {
        if !defaults.contains(rule)
            || issue.0.is_empty()
            || (tool.deferred.contains_key(rule) && tool.pending.contains_key(rule))
        {
            return Err(Error::Invalid(format!(
                "invalid or overlapping default disposition: {rule} -> {issue}"
            )));
        }
    }
    let mut supporting_seen = BTreeSet::new();
    let mut implemented = BTreeSet::new();
    let rules = directory.join(crate::domain::suite::constants::RULES);
    for rule in suite.rule_ids(ArtifactPath(&rules))? {
        if !defaults.contains(&rule) && !tool.supporting_rules.contains(&rule) {
            return Err(Error::Invalid(format!(
                "rule {rule} is not a shipped default"
            )));
        }
        let query = suite.rule_query(ArtifactPath(&rules), &rule)?;
        if query
            .as_ref()
            .is_some_and(|query| query.0.trim().is_empty())
        {
            return Err(Error::Invalid(format!("empty query for {rule}")));
        }
        if tool.supporting_rules.contains(&rule) {
            if query.is_none() {
                return Err(Error::Invalid(format!(
                    "supporting rule has no query: {rule}"
                )));
            }
            supporting_seen.insert(rule);
            continue;
        }
        let cases = suite.rule_cases(ArtifactPath(&rules), &rule)?;
        policy::validate_case_sources(&cases)?;
        if query.is_some() {
            policy::validate_cases(&rule, &cases)?;
            if tool.deferred.contains_key(&rule) || tool.pending.contains_key(&rule) {
                return Err(Error::Invalid(format!(
                    "authored rule still has a deferred or pending disposition: {rule}"
                )));
            }
            implemented.insert(rule);
        }
    }
    if supporting_seen.len() != tool.supporting_rules.len() {
        return Err(Error::Invalid(
            super::constants::MISSING_SUPPORTING_QUERY.into(),
        ));
    }
    policy::audited(
        tool,
        AuthoredInventory {
            missing: defaults.difference(&implemented).cloned().collect(),
            implemented: implemented.into_iter().collect(),
        },
    )
}
