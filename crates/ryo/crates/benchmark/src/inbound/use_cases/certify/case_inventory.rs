//! Registered native contexts reuse the sole shared inventory execution loop.
use crate::domain::{
    diagnostics::errors::DiagnosticsError as Error,
    suite::types::{CaseCount, Tool},
};
use crate::outbound::{
    execution::interface::ExecutionPort,
    inventory::{
        interface::InventoryPort,
        types::{Registration, RegistrationAuthority},
    },
    suite::{interface::SuitePort, types::ArtifactPath},
};
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
            "case inventory differs from measured complete rule identities".into(),
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
            "registered certification requires actual execution authority".into(),
        ));
    };
    if suite
        .digest(ArtifactPath(&measured_profile.join("benchmark.toml")))?
        .0
        != *profile_sha256
    {
        return Err(Error::Invalid(
            "registered authority differs from measured canonical profile".into(),
        ));
    }
    if profile.as_path() != measured_profile || rules != measured_profile.join("rules").as_path() {
        return Err(Error::Invalid(
            "registered profile and selected rule roots differ from measured authority".into(),
        ));
    }
    if suite.digest(ArtifactPath(reference))?.0 != reference_pin.sha256
        || suite.digest(ArtifactPath(candidate))?.0 != worker.sha256
    {
        return Err(Error::Invalid(
            "case inventory executable identities differ from measured contestants".into(),
        ));
    }
    let mut paths = vec![
        registration.to_owned(),
        profile.join("benchmark.toml"),
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
            .ok_or_else(|| Error::Invalid("missing verification command".into()))?;
        crate::application::run::registered_context::verify(entry, store)
            .map_err(Error::Invalid)?;
        let pin = entry
            .definition
            .as_ref()
            .ok_or_else(|| Error::Invalid("registered context lacks tested query".into()))?;
        if pin.path
            != context
                .join("rules")
                .join(&entry.rule_id)
                .join("query.ryoql")
            || entry.artifacts.get(&pin.path) != Some(&pin.sha256)
            || suite
                .digest(ArtifactPath(
                    &rules.join(&entry.rule_id).join("query.ryoql"),
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
            "registered independent case certification failed".into(),
        ));
    }
    Ok(CaseCount(
        registration.rules.iter().map(|rule| rule.cases.len()).sum(),
    ))
}
