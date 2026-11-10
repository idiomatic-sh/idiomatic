//! Empty in-memory suite preserves the distinction between defaults and implementations.
use crate::outbound::suite::interface::{ScopedDirectory, SuitePort};
use crate::outbound::suite::types::{
    ArtifactBytes, ArtifactPath, ArtifactSet, ArtifactSize, ArtifactText, MaterializedCase,
    MaterializedContext, RetainedDirectory, RuleQuery, SelectedName, SourceDigest,
};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::Suite;
use ryo_benchmark_contracts::domain::workloads::types::Tool;

/// The local adapter the fake leaves physical operations to.
fn local() -> impl SuitePort {
    crate::outbound::suite::factory::local()
}
/// The scratch directory the fake hands out, removed when it drops.
#[derive(Debug)]
pub struct ScopedDirectoryFake(tempfile::TempDir);
impl ScopedDirectory for ScopedDirectoryFake {
    fn path(&self) -> &std::path::Path {
        self.0.path()
    }
}
#[expect(
    clippy::partial_pub_fields,
    clippy::field_scoped_visibility_modifiers,
    reason = "crate-owned orchestration fixtures configure optional artifact availability; external callers supply prepared cases without forging internal source state"
)]
#[derive(Clone, Debug, Default)]
/// Deterministic suite substitute for orchestration and contract tests.
pub struct SuiteFake {
    /// The profile available from this deterministic artifact source.
    pub(crate) profile: Option<Tool>,
    /// The projects available from this deterministic artifact source.
    pub(crate) projects: Option<Suite>,
    /// Independent cases already materialized by the fixture.
    pub prepared: Vec<ryo_benchmark_contracts::domain::workloads::types::PreparedCase>,
    /// Explicit digest observations supplied by evidence-admission orchestration tests.
    pub(crate) artifact_digests: std::collections::BTreeMap<std::path::PathBuf, ArtifactDigest>,
    /// Whether this fixture has independently validated its declared source and evidence inputs.
    pub(crate) discrepancy_artifacts_valid: bool,
    /// Raw authored wire data for application conversion tests.
    pub(crate) reference_claims: Vec<crate::outbound::suite::types::CaseClaim>,
}
impl SuitePort for SuiteFake {
    fn validate_file_metadata(
        &self,
        request: &ryo_benchmark_contracts::domain::requests::types::RunRequest,
    ) -> Result<(), Error> {
        local().validate_file_metadata(request)
    }

    fn case_source_digest(
        &self,
        root: ArtifactPath<'_>,
        files: ArtifactSet<'_>,
    ) -> Result<SourceDigest, crate::outbound::suite::errors::SuiteError> {
        local().case_source_digest(root, files)
    }
    fn read_discrepancy_artifacts(
        &self,
        profile: ArtifactPath<'_>,
        artifacts: &[crate::outbound::suite::types::EvidenceArtifact],
    ) -> Result<
        std::collections::BTreeMap<std::path::PathBuf, ArtifactBytes>,
        crate::outbound::suite::errors::SuiteError,
    > {
        local().read_discrepancy_artifacts(profile, artifacts)
    }

    fn case_claims(
        &self,
        _: ArtifactPath<'_>,
        _: &[ryo::linter::RuleId],
    ) -> Result<
        Vec<crate::outbound::suite::types::CaseClaim>,
        crate::outbound::suite::errors::SuiteError,
    > {
        Ok(self.reference_claims.clone())
    }
    fn verify_discrepancy_artifacts(
        &self,
        _: crate::outbound::suite::types::CaseArtifacts<'_>,
    ) -> Result<(), crate::outbound::suite::errors::SuiteError> {
        if self.discrepancy_artifacts_valid {
            Ok(())
        } else {
            Err(crate::outbound::suite::errors::SuiteError::Invalid(
                "fixture has no independent discrepancy artifacts".into(),
            ))
        }
    }
    fn scoped_directory(
        &self,
        ArtifactPath(parent): ArtifactPath<'_>,
        ArtifactText(prefix): ArtifactText<'_>,
    ) -> Result<Box<dyn ScopedDirectory>, Error> {
        let mut builder = tempfile::Builder::new();
        let _prefixed = builder.prefix(prefix);
        let directory = if parent.as_os_str().is_empty() {
            builder.tempdir()?
        } else {
            builder.tempdir_in(parent)?
        };
        Ok(Box::new(ScopedDirectoryFake(directory)))
    }
    fn retained_directory(
        &self,
        parent: ArtifactPath<'_>,
        prefix: ArtifactText<'_>,
    ) -> Result<RetainedDirectory, Error> {
        local().retained_directory(parent, prefix)
    }
    fn copy_file(&self, from: ArtifactPath<'_>, to: ArtifactPath<'_>) -> Result<(), Error> {
        local().copy_file(from, to)
    }
    fn ensure_directory(&self, directory: ArtifactPath<'_>) -> Result<std::path::PathBuf, Error> {
        local().ensure_directory(directory)
    }
    fn write_text(&self, file: ArtifactPath<'_>, text: ArtifactText<'_>) -> Result<(), Error> {
        local().write_text(file, text)
    }
    fn accepted_differences(
        &self,
        file: ArtifactPath<'_>,
    ) -> Result<ryo_benchmark_contracts::domain::reports::types::AcceptedDifferences, Error> {
        local().accepted_differences(file)
    }
    fn source_text(&self, file: ArtifactPath<'_>) -> Result<ryo::linter::SourceText, Error> {
        local().source_text(file)
    }
    fn create_text(&self, file: ArtifactPath<'_>) -> Result<Box<dyn std::io::Write + Send>, Error> {
        local().create_text(file)
    }
    fn selection(&self, file: ArtifactPath<'_>) -> Result<Vec<SelectedName>, Error> {
        local().selection(file)
    }
    fn total_size(&self, files: ArtifactSet<'_>) -> Result<ArtifactSize, Error> {
        local().total_size(files)
    }
    fn digest(&self, ArtifactPath(path): ArtifactPath<'_>) -> Result<ArtifactDigest, Error> {
        self.artifact_digests
            .get(path)
            .cloned()
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound).into())
    }
    fn stage_projects(&self, _: ArtifactPath<'_>, _: ArtifactPath<'_>) -> Result<(), Error> {
        Err(std::io::Error::from(std::io::ErrorKind::NotFound).into())
    }
    fn stage(
        &self,
        _: ArtifactPath<'_>,
        _: ArtifactPath<'_>,
        _: &[ryo::linter::RuleId],
        _: &std::collections::BTreeSet<ryo::linter::RuleId>,
        _: ryo_benchmark_contracts::domain::workloads::types::StagePurpose,
    ) -> Result<(), Error> {
        Err(std::io::Error::from(std::io::ErrorKind::NotFound).into())
    }
    fn fingerprint(&self, _: ArtifactSet<'_>) -> Result<ArtifactDigest, Error> {
        Err(std::io::Error::from(std::io::ErrorKind::NotFound).into())
    }
    fn rule_ids(&self, _: ArtifactPath<'_>) -> Result<Vec<ryo::linter::RuleId>, Error> {
        Ok(Vec::new())
    }
    fn rule_query(
        &self,
        _: ArtifactPath<'_>,
        _: &ryo::linter::RuleId,
    ) -> Result<Option<RuleQuery>, Error> {
        Ok(None)
    }
    fn rule_cases(
        &self,
        _: ArtifactPath<'_>,
        _: &ryo::linter::RuleId,
    ) -> Result<ryo_benchmark_contracts::domain::workloads::types::Cases, Error> {
        Err(std::io::Error::from(std::io::ErrorKind::NotFound).into())
    }
    fn verify_query_modules(&self, _: ArtifactPath<'_>, _: &Tool) -> Result<(), Error> {
        Ok(())
    }
    fn materialize_case(
        &self,
        _: ArtifactPath<'_>,
        _: &ryo_benchmark_contracts::domain::workloads::types::Case,
        _: &MaterializedContext,
    ) -> Result<MaterializedCase, Error> {
        Err(std::io::Error::from(std::io::ErrorKind::NotFound).into())
    }
    fn materialize_inventory(
        &self,
        _: ArtifactPath<'_>,
        _: &ryo::linter::GivenProjectInventory,
    ) -> Result<(), Error> {
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
}
#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
