//! One common admission for declared input observations on both verifier routes.
use crate::outbound::suite::{interface::SuitePort, types::ArtifactPath};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::PreparedCase;
use std::path::Path;

pub(super) fn admit(
    suite: &dyn SuitePort,
    rules: &Path,
    cases: &[PreparedCase],
) -> Result<(), Error> {
    let root = rules
        .parent()
        .ok_or_else(|| Error::Invalid("missing case profile boundary".into()))?;
    let profile = suite.profile(ArtifactPath(root))?;
    for case in cases {
        crate::domain::suite::input_metadata::require(
            profile.file_observation,
            &case.request.files,
            case.request.file_metadata.as_ref(),
        )?;
    }
    Ok(())
}
