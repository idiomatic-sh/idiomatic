//! A worker runs only the target and phase identity its executable was built for.
use super::constants::{
    ACCEPTED_SYNTAX, ALWAYS, WORKER_PARSER_CONFIGURATION_DIFFER, WORKER_REQUIRES_EXPLICIT_SELECTED,
    WORKER_RULE_APPLICABILITY_DIFFERS,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::{
    ApplicabilityPolicy, ParserConfigurationName, ParserSelection, RuleLabel, WorkerRequest,
};
use std::collections::BTreeMap;

/// Bind the executable's parser and phase choices to the actual explicit request.
///
/// # Errors
/// Missing, unsupported or conflicting target and phase identities are not executions of a
/// canonical staged profile.
pub fn check(
    request: &WorkerRequest,
    expected: &ParserConfigurationName,
    pinned: &BTreeMap<RuleLabel, ApplicabilityPolicy>,
) -> Result<(), Error> {
    if request.parser_configuration != ParserSelection::Named(expected.clone()) {
        return Err(Error::Invalid(WORKER_PARSER_CONFIGURATION_DIFFER.into()));
    }
    if request.rule_applicability.is_empty() {
        return Err(Error::Invalid(WORKER_REQUIRES_EXPLICIT_SELECTED.into()));
    }
    for (rule, policy) in &request.rule_applicability {
        let admitted = matches!(policy.0.as_str(), ALWAYS | ACCEPTED_SYNTAX);
        let contradicts_pin = pinned.get(rule).is_some_and(|pin| pin != policy);
        if !admitted || contradicts_pin {
            return Err(Error::Invalid(WORKER_RULE_APPLICABILITY_DIFFERS.into()));
        }
    }
    Ok(())
}
