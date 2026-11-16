//! One common admission for declared input observations on both verifier routes.
use super::constants;
use crate::outbound::host::interface::HostPort;
use crate::outbound::suite::{interface::SuitePort, types::ArtifactPath};
use ryo_benchmark_contracts::domain::hosts::types::FilePermissions;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::FileObservation;
use ryo_benchmark_contracts::domain::workloads::types::PreparedCase;
use std::path::Path;

/// Admit the declared file observations of every case against the profile that pins them.
///
/// # Errors
/// Refuses a rules directory without a profile boundary, a profile that requires file observations on a host whose file permissions are unreliable, and observations the profile requires but a case omits.
pub fn admit(
    suite: &dyn SuitePort,
    host: &dyn HostPort,
    rules: &Path,
    cases: &[PreparedCase],
) -> Result<(), Error> {
    let root = rules
        .parent()
        .ok_or_else(|| Error::Invalid(constants::MISSING_CASE_PROFILE_BOUNDARY.into()))?;
    let profile = suite.profile(ArtifactPath(root))?;
    require_honoured_permissions(host, profile.file_observation)?;
    for case in cases {
        crate::domain::input_metadata::service::require(
            profile.file_observation,
            &case.request.files,
            &case.request.file_metadata,
        )?;
    }
    Ok(())
}

/// The one place that asks the host and decides: a profile that requires file observations cannot be verified or measured where the host's file permissions are unreliable.
///
/// # Errors
/// Refuses a required file observation on a host whose file permissions cannot be trusted.
fn require_honoured_permissions(
    host: &dyn HostPort,
    observation: FileObservation,
) -> Result<(), Error> {
    if observation != FileObservation::Undeclared
        && crate::domain::host_applicability::service::permissions(&host.facts())
            == FilePermissions::Unreliable
    {
        return Err(Error::Invalid(
            constants::HOST_DOES_NOT_HONOUR_FILE_PERMISSIONS.into(),
        ));
    }
    Ok(())
}

/// Admit a corpus run: every rule it selects is checked against the settings profile that configures it, through the same host decision as case admission.
///
/// # Errors
/// Refuses an unreadable settings profile, and a rule whose profile requires file observations on an unreliable host.
pub fn admit_rules(
    suite: &dyn SuitePort,
    host: &dyn HostPort,
    bench: &Path,
    rules: &[ryo::linter::RuleId],
) -> Result<(), Error> {
    for rule in rules {
        let directory = suite.settings_directory(ArtifactPath(bench), rule)?;
        let profile = suite.profile(ArtifactPath(&directory))?;
        require_honoured_permissions(host, profile.file_observation)?;
    }
    Ok(())
}
