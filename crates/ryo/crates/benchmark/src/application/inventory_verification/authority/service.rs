//! The `authority` application operation: a registration agrees with its pinned canonical profile, and each rule's context with its sealed artifacts.
use super::super::constants;
use crate::domain::suite::types::inventory::value::FailureReason;
use crate::outbound::{
    inventory::{
        interface::InventoryPort,
        types::{Command, RegisteredRule, Registration, RegistrationAuthority, RuleCommand},
    },
    suite::{interface::SuitePort, types::ArtifactPath},
};
use ryo_benchmark_contracts::domain::documents::types::text::value::RuleName;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::collections::BTreeSet;

/// A failure reason from a refusal's text.
pub(in crate::application::inventory_verification) fn refused(
    error: impl std::fmt::Display,
) -> FailureReason {
    FailureReason(error.to_string())
}

/// A complete result cannot contradict its pinned canonical profile.
///
/// # Errors
/// Refuses a registration whose authority, defaults or executables differ from the canonical profile.
pub fn require_authority(
    registration: &Registration,
    suite: &dyn SuitePort,
    _store: &dyn InventoryPort,
) -> Result<(), Error> {
    if matches!(
        &registration.authority,
        RegistrationAuthority::Unavailable { .. }
    ) && registration
        .rules
        .iter()
        .all(|rule| rule.command == RuleCommand::Unregistered)
    {
        return Ok(());
    }
    let RegistrationAuthority::Execution {
        profile,
        profile_sha256,
        reference,
        worker,
    } = &registration.authority
    else {
        return Err(Error::Invalid(
            constants::RECEIPT_ONLY_INVENTORY_CANNOT_AUTHORIZE_EXEC.into(),
        ));
    };
    let tool = suite.profile(ArtifactPath(profile))?;
    let observed_profile = suite.digest(ArtifactPath(
        &profile.join(ryo_benchmark_contracts::domain::requests::constants::PROFILE_MANIFEST),
    ))?;
    let observed_reference = suite.digest(ArtifactPath(&reference.path))?;
    let observed_worker = suite.digest(ArtifactPath(&worker.path))?;
    let declared: BTreeSet<_> = registration.defaults.iter().collect();
    let observed: Vec<RuleName> = tool
        .defaults
        .iter()
        .map(|rule| RuleName(rule.to_string()))
        .collect();
    let actual: BTreeSet<_> = observed.iter().collect();
    if declared.is_empty()
        || declared.len() != registration.defaults.len()
        || actual.len() != observed.len()
        || declared != actual
        || observed_profile.0 != profile_sha256.0
        || observed_reference.0 != reference.sha256.0
        || observed_worker.0 != worker.sha256.0
        || !tool
            .sha256
            .iter()
            .any(|approved| approved.0 == reference.sha256.0)
    {
        return Err(Error::Invalid(
            constants::CANONICAL_INVENTORY_PROFILE_EXECUTABLE_IDENT.into(),
        ));
    }
    for entry in &registration.rules {
        let name = entry.rule_id.0.as_str();
        if entry.command != RuleCommand::Unregistered
            && (tool.deferred.keys().any(|rule| rule.to_string() == name)
                || tool.pending.keys().any(|rule| rule.to_string() == name))
        {
            return Err(Error::Invalid(format!(
                "canonical profile defers an executable registered rule: {name}"
            )));
        }
    }
    Ok(())
}

/// Validate the full context and its exhaustive module declarations, before and after native execution.
///
/// # Errors
/// Refuses a rule without a verification command, a context that differs from its pins, and a module inventory or pin that differs from the profile.
pub fn verify_context(
    entry: &RegisteredRule,
    suite: &dyn SuitePort,
    store: &dyn InventoryPort,
) -> Result<(), FailureReason> {
    let RuleCommand::Registered(Command::CaseVerification { context }) = &entry.command else {
        return Err(FailureReason(
            crate::domain::suite::constants::MISSING_VERIFICATION_COMMAND.into(),
        ));
    };
    if entry.artifacts.is_empty() {
        return Err(FailureReason(
            constants::REGISTERED_CASE_CONTEXT_IS_EMPTY.into(),
        ));
    }
    let listed = store.context_files(context).map_err(refused)?;
    if listed.files != entry.artifacts.keys().cloned().collect::<BTreeSet<_>>() {
        return Err(FailureReason(
            constants::REGISTERED_CONTEXT_NAMESPACE_DIFFERS_FROM_PI.into(),
        ));
    }
    for (path, expected) in &entry.artifacts {
        let actual = suite.digest(ArtifactPath(path)).map_err(refused)?;
        if actual.0 != expected.0 {
            return Err(FailureReason(format!(
                "registered case artifact changed: {}",
                path.display()
            )));
        }
    }
    let tool = suite.profile(ArtifactPath(context)).map_err(refused)?;
    if !tool
        .query_modules
        .keys()
        .map(|name| name.0.as_str())
        .eq(entry.modules.keys().map(|name| name.0.as_str()))
    {
        return Err(FailureReason(
            constants::REGISTERED_MODULE_INVENTORY_DIFFERS_FROM_PIN.into(),
        ));
    }
    for (name, relative) in &tool.query_modules {
        let pin = entry
            .modules
            .iter()
            .find(|(declared, _)| declared.0 == name.0)
            .map(|(_, pin)| pin)
            .ok_or_else(|| FailureReason(constants::MISSING_DECLARED_MODULE_PIN.into()))?;
        if pin.path != context.join(relative) || entry.artifacts.get(&pin.path) != Some(&pin.sha256)
        {
            return Err(FailureReason(format!(
                "registered module pin differs from pinned context profile: {}",
                name.0
            )));
        }
    }
    Ok(())
}
