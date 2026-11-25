//! Registered cases certify only the code that is actually measured.
use super::constants;
use crate::outbound::{
    inventory::types::{
        Command, RegisteredRule, Registration, RegistrationAuthority, RuleCommand, RuleDefinition,
        RuleReason,
    },
    suite::{interface::SuitePort, types::ArtifactPath},
};
use benchmark_contracts::domain::reports::errors::ReportsError as Error;
use benchmark_contracts::domain::requests::types::RegisteredCoverage;
use benchmark_contracts::domain::workloads::types::Tool;
use std::path::{Path, PathBuf};

/// Require the registration to bind exactly the measured profile, rules and contestants.
///
/// A selected coverage binds its rules to the measured defaults and its source to the registered profile.
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
    coverage: &RegisteredCoverage,
) -> Result<Vec<PathBuf>, Error> {
    require_defaults(loaded, tool, coverage)?;
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
    match coverage {
        RegisteredCoverage::Full => {
            // A registration with deferred defaults is measured on the profile staging narrows from the registered one in
            // this process, whose own bytes the certificate seals: the registered profile itself is what must be unchanged.
            let deferred = loaded
                .rules
                .iter()
                .any(|rule| matches!(rule.command, RuleCommand::Deferred(_)));
            let manifest = if deferred { profile } else { measured_profile };
            if suite
                .digest(ArtifactPath(&manifest.join(
                    benchmark_contracts::domain::requests::constants::PROFILE_MANIFEST,
                )))?
                .0
                != profile_sha256.0
            {
                return Err(Error::Invalid(
                    constants::REGISTERED_AUTHORITY_DIFFERS_FROM_MEASURED_C.into(),
                ));
            }
        }
        // The narrowed measured manifest is staged from `source` in this process; its own bytes are sealed by the certificate.
        RegisteredCoverage::Selected { source, .. } => {
            if source != profile
                || suite
                    .digest(ArtifactPath(&source.join(
                        benchmark_contracts::domain::requests::constants::PROFILE_MANIFEST,
                    )))?
                    .0
                    != profile_sha256.0
            {
                return Err(Error::Invalid(
                    constants::SELECTED_SOURCE_DIFFERS_FROM_AUTHORITY.into(),
                ));
            }
        }
    }
    if rules
        != measured_profile
            .join(crate::domain::suite::constants::RULES)
            .as_path()
    {
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
        profile.join(benchmark_contracts::domain::requests::constants::PROFILE_MANIFEST),
        reference_pin.path.clone(),
        worker.path.clone(),
    ])
}

/// The registered rules certification verifies: those the coverage covers but the deferred ones, which have no query and no
/// verification command; the registration accounts for them and nothing certifies them.
pub(crate) fn certified_rules<'a>(
    coverage: &'a RegisteredCoverage,
    rules: &'a [RegisteredRule],
) -> impl Iterator<Item = &'a RegisteredRule> + 'a {
    crate::domain::suite::service::covered(coverage, rules, |entry| &entry.rule_id)
        .filter(|entry| !matches!(entry.command, RuleCommand::Deferred(_)))
}

/// Require the registration's defaults to be the measured profile's: all of them, but those registered as deferred, for full
/// coverage; the selected ones, each a registered default, for a selection.
///
/// # Errors
/// Refuses defaults that differ from what is measured.
pub(crate) fn require_defaults(
    loaded: &Registration,
    tool: &Tool,
    coverage: &RegisteredCoverage,
) -> Result<(), Error> {
    let expected: std::collections::BTreeSet<_> =
        tool.defaults.iter().map(ToString::to_string).collect();
    let actual: std::collections::BTreeSet<_> =
        loaded.defaults.iter().map(|rule| rule.0.clone()).collect();
    match coverage {
        RegisteredCoverage::Full => {
            // The measured profile holds the registered defaults but those registered as deferred: a deferred default has no
            // query, so staging leaves it out, and the registration already holds each deferral to its capability.
            let deferred: std::collections::BTreeSet<_> = loaded
                .rules
                .iter()
                .filter(|rule| matches!(rule.command, RuleCommand::Deferred(_)))
                .map(|rule| rule.rule_id.0.clone())
                .collect();
            let measured: std::collections::BTreeSet<_> =
                actual.difference(&deferred).cloned().collect();
            if expected != measured || loaded.defaults.len() != expected.len() + deferred.len() {
                return Err(Error::Invalid(
                    constants::CASE_INVENTORY_DIFFERS_FROM_MEASURED.into(),
                ));
            }
        }
        RegisteredCoverage::Selected { rules: chosen, .. } => {
            let selected: std::collections::BTreeSet<_> =
                chosen.iter().map(ToString::to_string).collect();
            if selected.is_empty()
                || selected.len() != chosen.len()
                || selected != expected
                || !selected.is_subset(&actual)
            {
                return Err(Error::Invalid(
                    constants::SELECTED_RULES_DIFFER_FROM_MEASURED.into(),
                ));
            }
        }
    }
    Ok(())
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
