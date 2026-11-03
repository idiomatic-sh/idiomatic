//! Empty in-memory suite preserves the distinction between defaults and implementations.
use crate::domain::suite::types::ArtifactDigest;
use crate::domain::{
    diagnostics::errors::DiagnosticsError as Error,
    suite::types::{Suite, Tool, audit::value::Audit},
};
use crate::outbound::suite::interface::SuitePort;
use crate::outbound::suite::types::{ArtifactPath, ArtifactSet};
#[expect(
    clippy::partial_pub_fields,
    clippy::field_scoped_visibility_modifiers,
    reason = "crate-owned orchestration fixtures configure optional artifact availability; external callers supply prepared cases without forging internal source state"
)]
#[derive(Clone, Debug, Default)]
/// Deterministic suite substitute for orchestration and contract tests.
pub struct SuiteFake {
    /// Per-instance receipt output selected by orchestration tests.
    pub(crate) case_receipt: Option<std::path::PathBuf>,
    /// The profile available from this deterministic artifact source.
    pub(crate) profile: Option<Tool>,
    /// The projects available from this deterministic artifact source.
    pub(crate) projects: Option<Suite>,
    /// Independent cases already materialized by the fixture.
    pub prepared: Vec<crate::domain::suite::types::PreparedCase>,
    /// Explicit digest observations supplied by evidence-admission orchestration tests.
    pub(crate) artifact_digests: std::collections::BTreeMap<std::path::PathBuf, ArtifactDigest>,
    /// Whether this fixture has independently validated its declared source and evidence inputs.
    pub(crate) discrepancy_artifacts_valid: bool,
    /// Raw authored wire data for application conversion tests.
    pub(crate) reference_wires: Vec<crate::outbound::suite::types::discrepancy::CaseReferenceWire>,
}
impl SuitePort for SuiteFake {
    fn case_receipt_file(&self) -> Option<std::path::PathBuf> {
        self.case_receipt.clone()
    }

    fn case_source_digest(
        &self,
        root: ArtifactPath<'_>,
        files: &[std::path::PathBuf],
    ) -> Result<String, crate::outbound::suite::types::discrepancy::ArtifactError> {
        crate::outbound::suite::factory::local().case_source_digest(root, files)
    }
    fn read_discrepancy_artifacts(
        &self,
        profile: ArtifactPath<'_>,
        artifacts: &[crate::outbound::suite::types::discrepancy::EvidenceArtifact],
    ) -> Result<
        std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>,
        crate::outbound::suite::types::discrepancy::ArtifactError,
    > {
        crate::outbound::suite::factory::local().read_discrepancy_artifacts(profile, artifacts)
    }

    fn record_case(
        &self,
        receipt: ArtifactPath<'_>,
        record: serde_json::Value,
        process: Option<crate::outbound::suite::types::process::CapturedProcess<'_>>,
    ) -> Result<(), crate::outbound::suite::types::discrepancy::ArtifactError> {
        crate::outbound::suite::factory::local().record_case(receipt, record, process)
    }

    fn case_reference_wires(
        &self,
        _: ArtifactPath<'_>,
    ) -> Result<
        Vec<crate::outbound::suite::types::discrepancy::CaseReferenceWire>,
        crate::outbound::suite::types::discrepancy::ArtifactError,
    > {
        Ok(self.reference_wires.clone())
    }
    fn verify_discrepancy_artifacts(
        &self,
        _: crate::outbound::suite::types::discrepancy::CaseArtifacts<'_>,
    ) -> Result<(), crate::outbound::suite::types::discrepancy::ArtifactError> {
        if self.discrepancy_artifacts_valid {
            Ok(())
        } else {
            Err(
                crate::outbound::suite::types::discrepancy::ArtifactError::Invalid(
                    "fixture has no independent discrepancy artifacts".into(),
                ),
            )
        }
    }
    fn digest(&self, ArtifactPath(path): ArtifactPath<'_>) -> Result<ArtifactDigest, Error> {
        self.artifact_digests
            .get(path)
            .cloned()
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound).into())
    }
    fn cases(
        &self,
        _: ArtifactPath<'_>,
        _: ArtifactPath<'_>,
    ) -> Result<Vec<crate::domain::suite::types::PreparedCase>, Error> {
        if self.prepared.is_empty() {
            Err(Error::Invalid("no executable rule cases".into()))
        } else {
            Ok(self.prepared.clone())
        }
    }
    fn stage_projects(&self, _: ArtifactPath<'_>, _: ArtifactPath<'_>) -> Result<(), Error> {
        Err(std::io::Error::from(std::io::ErrorKind::NotFound).into())
    }
    fn stage(&self, _: ArtifactPath<'_>, _: ArtifactPath<'_>, _: &Audit) -> Result<(), Error> {
        Err(std::io::Error::from(std::io::ErrorKind::NotFound).into())
    }
    fn fingerprint(&self, _: ArtifactSet<'_>) -> Result<ArtifactDigest, Error> {
        Err(std::io::Error::from(std::io::ErrorKind::NotFound).into())
    }
    fn profile(&self, _: ArtifactPath<'_>) -> Result<Tool, Error> {
        self.profile
            .clone()
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound).into())
    }
    fn projects(&self, _: ArtifactPath<'_>) -> Result<Suite, Error> {
        self.projects
            .clone()
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound).into())
    }
    fn verify_tool(&self, _: ArtifactPath<'_>, tool: &Tool) -> Result<(), Error> {
        if self.profile.is_none() {
            return Err(std::io::Error::from(std::io::ErrorKind::NotFound).into());
        }
        match &self.profile {
            Some(expected)
                if expected.tool == tool.tool
                    && expected.version == tool.version
                    && expected.sha256 == tool.sha256
                    && expected.defaults == tool.defaults =>
            {
                Ok(())
            }
            _ => Err(Error::Invalid(
                "tool does not match configured fixture profile".into(),
            )),
        }
    }

    fn audit(&self, _: ArtifactPath<'_>, tool: &Tool) -> Result<Audit, Error> {
        if tool.defaults.is_empty() {
            return Err(Error::Invalid(
                "empty or duplicated default rule inventory".into(),
            ));
        }
        let (implemented, missing) = tool
            .defaults
            .iter()
            .cloned()
            .partition(|rule| self.prepared.iter().any(|case| &case.rule == rule));
        Ok(Audit {
            implemented,
            missing,
        })
    }
}
#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
