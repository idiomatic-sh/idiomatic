//! Registered cases certify only the code that is actually measured.
use super::constants;
use crate::outbound::{
    inventory::types::{
        Command, RegisteredRule, Registration, RegistrationAuthority, RuleCommand, RuleDefinition,
        RuleReason,
    },
    suite::{interface::SuitePort, types::ArtifactPath},
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::Tool;
use std::path::{Path, PathBuf};

/// Require the registration to bind exactly the measured profile, rules and contestants.
///
/// # Errors
/// Refuses defaults, authority, rule root or executable identities that differ from what is measured.
pub fn admit(
    loaded: &Registration,
    suite: &dyn SuitePort,
    registration: &Path,
    measured_profile: &Path,
    tool: &Tool,
    rules: &Path,
    reference: &Path,
    candidate: &Path,
) -> Result<Vec<PathBuf>, Error> {
    let expected: std::collections::BTreeSet<_> =
        tool.defaults.iter().map(ToString::to_string).collect();
    let actual: std::collections::BTreeSet<_> =
        loaded.defaults.iter().map(|rule| rule.0.clone()).collect();
    if expected != actual || loaded.defaults.len() != expected.len() {
        return Err(Error::Invalid(
            constants::CASE_INVENTORY_DIFFERS_FROM_MEASURED_COMPLET.into(),
        ));
    }
    let RegistrationAuthority::Execution {
        profile,
        reference: reference_pin,
        worker,
        profile_sha256,
    } = &loaded.authority
    else {
        return Err(Error::Invalid(
            constants::REGISTERED_CERTIFICATION_REQUIRES_ACTUAL_EXE.into(),
        ));
    };
    if suite
        .digest(ArtifactPath(
            &measured_profile.join(ryo_benchmark_contracts::domain::requests::constants::PROFILE_MANIFEST),
        ))?
        .0
        != profile_sha256.0
    {
        return Err(Error::Invalid(
            constants::REGISTERED_AUTHORITY_DIFFERS_FROM_MEASURED_C.into(),
        ));
    }
    if rules != measured_profile.join(crate::domain::suite::constants::RULES).as_path() {
        return Err(Error::Invalid(
            constants::SELECTED_RULE_ROOT_DIFFERS_FROM_MEASURED.into(),
        ));
    }
    if suite.digest(ArtifactPath(reference))?.0 != reference_pin.sha256.0
        || suite.digest(ArtifactPath(candidate))?.0 != worker.sha256.0
    {
        return Err(Error::Invalid(
            constants::CASE_INVENTORY_EXECUTABLE_IDENTITIES_DIFFER.into(),
        ));
    }
    Ok(vec![
        registration.to_owned(),
        profile.join(ryo_benchmark_contracts::domain::requests::constants::PROFILE_MANIFEST),
        reference_pin.path.clone(),
        worker.path.clone(),
    ])
}

/// Require one registered rule's tested query and helpers to be the measured ones.
///
/// # Errors
/// Refuses an unexecutable rule, and a query or helper that differs from the measured profile.
pub fn pin_rule(
    entry: &RegisteredRule,
    suite: &dyn SuitePort,
    measured_profile: &Path,
    tool: &Tool,
    rules: &Path,
) -> Result<Vec<PathBuf>, Error> {
    let (RuleReason::Unstated, RuleCommand::Registered(Command::CaseVerification { context })) =
        (&entry.reason, &entry.command)
    else {
        return Err(Error::Invalid(format!(
            "unexecutable registered context: {}",
            entry.rule_id.0
        )));
    };
    let RuleDefinition::Pinned(pin) = &entry.definition else {
        return Err(Error::Invalid(
            constants::REGISTERED_CONTEXT_LACKS_TESTED_QUERY.into(),
        ));
    };
    if pin.path
        != context
            .join(crate::domain::suite::constants::RULES)
            .join(&entry.rule_id.0)
            .join(crate::domain::suite::constants::QUERY_RYOQL)
        || entry.artifacts.get(&pin.path) != Some(&pin.sha256)
        || suite
            .digest(ArtifactPath(
                &rules
                    .join(&entry.rule_id.0)
                    .join(crate::domain::suite::constants::QUERY_RYOQL),
            ))?
            .0
            != pin.sha256.0
    {
        return Err(Error::Invalid(format!(
            "measured query differs from registered context: {}",
            entry.rule_id.0
        )));
    }
    for (name, pin) in &entry.modules {
        let relative = tool
            .query_modules
            .iter()
            .find(|(declared, _)| declared.0 == name.0)
            .map(|(_, relative)| relative)
            .ok_or_else(|| {
                Error::Invalid(format!("measured profile lacks tested module: {}", name.0))
            })?;
        if entry.artifacts.get(&pin.path) != Some(&pin.sha256)
            || suite
                .digest(ArtifactPath(&measured_profile.join(relative)))?
                .0
                != pin.sha256.0
        {
            return Err(Error::Invalid(format!(
                "measured helper differs from tested context: {}",
                name.0
            )));
        }
    }
    Ok(entry.artifacts.keys().cloned().collect())
}
