//! One common admission for declared input observations on both verifier routes.
use super::constants;
use crate::outbound::suite::{interface::SuitePort, types::ArtifactPath};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::PreparedCase;
use std::path::Path;

/// Admit the declared file observations of every case against the profile that pins them.
///
/// # Errors
/// Refuses a rules directory without a profile boundary and observations the profile requires but a case omits.
pub fn admit(suite: &dyn SuitePort, rules: &Path, cases: &[PreparedCase]) -> Result<(), Error> {
    let root = rules
        .parent()
        .ok_or_else(|| Error::Invalid(constants::MISSING_CASE_PROFILE_BOUNDARY.into()))?;
    let profile = suite.profile(ArtifactPath(root))?;
    for case in cases {
        crate::domain::input_metadata::service::require(
            profile.file_observation,
            &case.request.files,
            &case.request.file_metadata,
        )?;
    }
    Ok(())
}
