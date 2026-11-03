//! One common admission for declared input observations on both verifier routes.
use crate::domain::{diagnostics::errors::DiagnosticsError as Error, suite::types::PreparedCase};
use crate::outbound::suite::{interface::SuitePort, types::ArtifactPath};
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
