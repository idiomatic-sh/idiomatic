//! Registered native contexts reuse the sole shared inventory execution loop.
use crate::inbound::use_cases::certify::constants;
use crate::outbound::{
    execution::interface::ExecutionPort,
    inventory::{
        interface::InventoryPort,
        types::{Registration, RegistrationAuthority},
    },
    suite::{interface::SuitePort, types::ArtifactPath},
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::Tool;
use ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount;
use std::path::{Path, PathBuf};
pub(super) enum PreparedCases {
    Profile,
    Registered {
        registration: Registration,
        output: PathBuf,
    },
}
/// Preserve every registered context, and require measured code to be the code actually tested.
pub(super) fn prepare(
    store: &dyn InventoryPort,
    suite: &dyn SuitePort,
    registration: &Path,
    measured_profile: &Path,
    tool: &Tool,
    rules: &Path,
    reference: &Path,
    candidate: &Path,
) -> Result<(Registration, Vec<PathBuf>), Error> {
    let loaded = store.load(registration).map_err(Error::Invalid)?;
    crate::application::run::inventory::require_authority(&loaded, store)?;
    let expected: std::collections::BTreeSet<_> =
        tool.defaults.iter().map(ToString::to_string).collect();
    let actual: std::collections::BTreeSet<_> = loaded.defaults.iter().cloned().collect();
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
            &measured_profile.join(constants::FIXTURE_PROFILE_FILE),
        ))?
        .0
        != *profile_sha256
    {
        return Err(Error::Invalid(
            constants::REGISTERED_AUTHORITY_DIFFERS_FROM_MEASURED_C.into(),
        ));
    }
    if rules
        != measured_profile
            .join(constants::FIXTURE_RULE_DIRECTORY_NAME)
            .as_path()
    {
        return Err(Error::Invalid(
            constants::SELECTED_RULE_ROOT_DIFFERS_FROM_MEASURED.into(),
        ));
    }
    if suite.digest(ArtifactPath(reference))?.0 != reference_pin.sha256
        || suite.digest(ArtifactPath(candidate))?.0 != worker.sha256
    {
        return Err(Error::Invalid(
            constants::CASE_INVENTORY_EXECUTABLE_IDENTITIES_DIFFER.into(),
        ));
    }
    let mut paths = vec![
        registration.to_owned(),
        profile.join(constants::FIXTURE_PROFILE_FILE),
        reference_pin.path.clone(),
        worker.path.clone(),
    ];
    for entry in &loaded.rules {
        if entry.reason.is_some() || entry.command.is_none() {
            return Err(Error::Invalid(format!(
                "unexecutable registered context: {}",
                entry.rule_id
            )));
        }
        let crate::outbound::inventory::types::Command::CaseVerification { context } = entry
            .command
            .as_ref()
            .ok_or_else(|| Error::Invalid(constants::MISSING_VERIFICATION_COMMAND.into()))?;
        crate::application::run::registered_context::verify(entry, store)
            .map_err(Error::Invalid)?;
        let pin = entry.definition.as_ref().ok_or_else(|| {
            Error::Invalid(constants::REGISTERED_CONTEXT_LACKS_TESTED_QUERY.into())
        })?;
        if pin.path
            != context
                .join(constants::FIXTURE_RULE_DIRECTORY_NAME)
                .join(&entry.rule_id)
                .join(constants::QUERY_RYOQL)
            || entry.artifacts.get(&pin.path) != Some(&pin.sha256)
            || suite
                .digest(ArtifactPath(
                    &rules.join(&entry.rule_id).join(constants::QUERY_RYOQL),
                ))?
                .0
                != pin.sha256
        {
            return Err(Error::Invalid(format!(
                "measured query differs from registered context: {}",
                entry.rule_id
            )));
        }
        for (name, pin) in &entry.modules {
            let relative = tool.query_modules.get(name).ok_or_else(|| {
                Error::Invalid(format!("measured profile lacks tested module: {name}"))
            })?;
            if entry.artifacts.get(&pin.path) != Some(&pin.sha256)
                || suite
                    .digest(ArtifactPath(&measured_profile.join(relative)))?
                    .0
                    != pin.sha256
            {
                return Err(Error::Invalid(format!(
                    "measured helper differs from tested context: {name}"
                )));
            }
        }
        paths.extend(entry.artifacts.keys().cloned());
    }
    paths.sort();
    paths.dedup();
    Ok((loaded, paths))
}
/// Certify once; cold/warm/edit measurements reuse the sealed certificate.
pub(super) fn execute(
    registration: &Registration,
    output: &Path,
    execution: &dyn ExecutionPort,
    store: &dyn InventoryPort,
) -> Result<CaseCount, Error> {
    if !crate::application::run::inventory::verify(registration, output, execution, store)? {
        return Err(Error::Invalid(
            constants::REGISTERED_INDEPENDENT_CASE_CERTIFICATION_FA.into(),
        ));
    }
    Ok(CaseCount(
        registration.rules.iter().map(|rule| rule.cases.len()).sum(),
    ))
}
