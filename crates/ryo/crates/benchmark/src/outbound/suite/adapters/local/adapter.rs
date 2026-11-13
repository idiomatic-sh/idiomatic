//! Validate pins, cases and rule coverage before running contestants.
use super::constants;
use super::evidence::schemas::verify_requirement;
use super::writer::{fingerprint_path, relative, rule_directory, validate_query_modules};
use crate::outbound::suite::errors::SuiteError;
use crate::outbound::suite::interface::{ScopedDirectory, SuitePort};
use crate::outbound::suite::types::{
    ArtifactBytes, ArtifactMetadata, ArtifactPath, ArtifactSet, ArtifactSize, ArtifactText,
    CaseArtifacts, MaterializedCase, MaterializedContext, RetainedDirectory, RuleQuery,
    SourceDigest,
};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::Cases;
use ryo_benchmark_contracts::domain::workloads::types::Tool;
use sha2::{Digest as _, Sha256};
use std::collections::BTreeSet;
use std::path::Path;

/// Hash one regular file without directory framing.
fn digest(path: &Path) -> Result<ArtifactDigest, Error> {
    if !std::fs::symlink_metadata(path)?.is_file() {
        return Err(Error::Invalid(constants::INVALID_DIGEST_KIND.into()));
    }
    Ok(ArtifactDigest(format!(
        "{:x}",
        Sha256::digest(std::fs::read(path)?)
    )))
}

/// Verify executable bytes before checking the exact version response.
fn verify_executable(path: &Path, tool: &Tool) -> Result<(), Error> {
    let actual = digest(path)?;
    if !tool.sha256.iter().any(|expected| expected.0 == actual.0) {
        return Err(Error::Invalid(constants::TOOL_HASH_MISMATCH.into()));
    }
    let output = std::process::Command::new(path)
        .arg(constants::VERSION_ARGUMENT)
        .output()?;
    if !output.status.success()
        || String::from_utf8_lossy(&output.stdout).trim()
            != format!("{} {}", tool.tool, tool.version)
    {
        return Err(Error::Invalid(constants::TOOL_VERSION_MISMATCH.into()));
    }
    Ok(())
}

/// The scratch directory a temporary directory handle removes when it drops.
struct LocalScopedDirectory(tempfile::TempDir);
impl ScopedDirectory for LocalScopedDirectory {
    fn path(&self) -> &Path {
        self.0.path()
    }
}

#[derive(Debug)]
/// Reads and freezes validated benchmark artifacts from the local filesystem.
pub(crate) struct SuiteLocalAdapter;
/// Make a fresh directory beneath the parent, or beneath the system temporary directory for an empty parent.
fn scratch_builder(
    ArtifactPath(parent): ArtifactPath<'_>,
    ArtifactText(prefix): ArtifactText<'_>,
) -> Result<tempfile::TempDir, Error> {
    let mut builder = tempfile::Builder::new();
    let _prefixed = builder.prefix(prefix);
    let directory = if parent.as_os_str().is_empty() {
        builder.tempdir()?
    } else {
        builder.tempdir_in(parent)?
    };
    Ok(directory)
}

impl SuitePort for SuiteLocalAdapter {
    fn validate_file_metadata(
        &self,
        request: &ryo_benchmark_contracts::domain::requests::types::RunRequest,
    ) -> Result<(), Error> {
        super::file_metadata::schemas::validate(request)
    }

    fn case_source_digest(
        &self,
        root: ArtifactPath<'_>,
        files: ArtifactSet<'_>,
    ) -> Result<SourceDigest, crate::outbound::suite::errors::SuiteError> {
        source_digest(root.0, files.0)
    }

    fn read_discrepancy_artifacts(
        &self,
        profile: ArtifactPath<'_>,
        artifacts: &[crate::outbound::suite::types::EvidenceArtifact],
    ) -> Result<
        std::collections::BTreeMap<std::path::PathBuf, ArtifactBytes>,
        crate::outbound::suite::errors::SuiteError,
    > {
        read(profile.0, artifacts)
    }
    fn case_claims(
        &self,
        ArtifactPath(profile): ArtifactPath<'_>,
        owners: &[ryo::linter::RuleId],
    ) -> Result<
        Vec<crate::outbound::suite::types::CaseClaim>,
        crate::outbound::suite::errors::SuiteError,
    > {
        super::writer::case_claims(profile, owners)
    }
    fn verify_discrepancy_artifacts(
        &self,
        artifacts: crate::outbound::suite::types::CaseArtifacts<'_>,
    ) -> Result<(), crate::outbound::suite::errors::SuiteError> {
        verify(artifacts)
    }
    fn scoped_directory(
        &self,
        parent: ArtifactPath<'_>,
        prefix: ArtifactText<'_>,
    ) -> Result<Box<dyn ScopedDirectory>, Error> {
        Ok(Box::new(LocalScopedDirectory(scratch_builder(
            parent, prefix,
        )?)))
    }
    fn retained_directory(
        &self,
        parent: ArtifactPath<'_>,
        prefix: ArtifactText<'_>,
    ) -> Result<RetainedDirectory, Error> {
        Ok(RetainedDirectory(scratch_builder(parent, prefix)?.keep()))
    }
    fn copy_file(
        &self,
        ArtifactPath(from): ArtifactPath<'_>,
        ArtifactPath(to): ArtifactPath<'_>,
    ) -> Result<(), Error> {
        std::fs::copy(from, to).map(|_bytes| ()).map_err(Into::into)
    }
    fn ensure_directory(
        &self,
        ArtifactPath(directory): ArtifactPath<'_>,
    ) -> Result<std::path::PathBuf, Error> {
        std::fs::create_dir_all(directory)?;
        directory.canonicalize().map_err(Into::into)
    }
    fn write_text(
        &self,
        ArtifactPath(file): ArtifactPath<'_>,
        ArtifactText(text): ArtifactText<'_>,
    ) -> Result<(), Error> {
        std::fs::write(file, text).map_err(Into::into)
    }
    fn accepted_differences(
        &self,
        ArtifactPath(file): ArtifactPath<'_>,
    ) -> Result<ryo_benchmark_contracts::domain::reports::types::AcceptedDifferences, Error> {
        super::accepted::schemas::accepted(&std::fs::read(file)?)
    }
    fn source_text(
        &self,
        ArtifactPath(file): ArtifactPath<'_>,
    ) -> Result<ryo::linter::SourceText, Error> {
        Ok(ryo::linter::SourceText(
            std::fs::read_to_string(file)?.into_boxed_str(),
        ))
    }
    fn create_text(
        &self,
        ArtifactPath(file): ArtifactPath<'_>,
    ) -> Result<Box<dyn std::io::Write + Send>, Error> {
        Ok(Box::new(std::fs::File::create(file)?))
    }
    fn total_size(&self, ArtifactSet(files): ArtifactSet<'_>) -> Result<ArtifactSize, Error> {
        files
            .iter()
            .map(|file| std::fs::metadata(file).map(|metadata| metadata.len()))
            .sum::<Result<u64, _>>()
            .map(ArtifactSize)
            .map_err(Into::into)
    }
    fn digest(&self, ArtifactPath(path): ArtifactPath<'_>) -> Result<ArtifactDigest, Error> {
        digest(path)
    }
    fn rule_ids(
        &self,
        ArtifactPath(profile): ArtifactPath<'_>,
    ) -> Result<Vec<ryo::linter::RuleId>, Error> {
        std::fs::read_dir(profile)?
            .map(|entry| {
                let name = entry?
                    .file_name()
                    .into_string()
                    .map_err(|_| Error::Invalid(constants::INVALID_RULE_ID.into()))?;
                super::rule_cases::schemas::rule(&name)
            })
            .collect()
    }
    fn rule_query(
        &self,
        ArtifactPath(profile): ArtifactPath<'_>,
        rule: &ryo::linter::RuleId,
    ) -> Result<Option<RuleQuery>, Error> {
        match std::fs::read_to_string(rule_directory(profile, rule)?.join(constants::QUERY)) {
            Ok(query) => Ok(Some(RuleQuery(query))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
    fn rule_cases(
        &self,
        ArtifactPath(profile): ArtifactPath<'_>,
        rule: &ryo::linter::RuleId,
    ) -> Result<Cases, Error> {
        super::rule_cases::schemas::cases(&std::fs::read_to_string(
            rule_directory(profile, rule)?.join(constants::CASES),
        )?)
    }
    fn verify_query_modules(
        &self,
        ArtifactPath(profile): ArtifactPath<'_>,
        tool: &Tool,
    ) -> Result<(), Error> {
        validate_query_modules(profile, tool)
    }
    fn materialize_case(
        &self,
        ArtifactPath(destination): ArtifactPath<'_>,
        case: &ryo_benchmark_contracts::domain::workloads::types::Case,
        context: &MaterializedContext,
    ) -> Result<MaterializedCase, Error> {
        super::writer::materialize(destination, case, context)
    }
    fn materialize_inventory(
        &self,
        ArtifactPath(root): ArtifactPath<'_>,
        inventory: &ryo::linter::GivenProjectInventory,
    ) -> Result<(), Error> {
        super::writer::materialize_inventory(root, inventory)
    }
    fn stage_projects(
        &self,
        ArtifactPath(source): ArtifactPath<'_>,
        ArtifactPath(destination): ArtifactPath<'_>,
    ) -> Result<(), Error> {
        super::writer::stage_projects(source, destination)
    }
    fn stage(
        &self,
        ArtifactPath(directory): ArtifactPath<'_>,
        ArtifactPath(destination): ArtifactPath<'_>,
        authored: &[ryo::linter::RuleId],
        definitions: &BTreeSet<ryo::linter::RuleId>,
        purpose: ryo_benchmark_contracts::domain::workloads::types::StagePurpose,
    ) -> Result<(), Error> {
        super::writer::stage(directory, destination, authored, definitions, purpose)
    }
    fn fingerprint(&self, ArtifactSet(paths): ArtifactSet<'_>) -> Result<ArtifactDigest, Error> {
        let mut hasher = Sha256::new();
        for path in paths {
            fingerprint_path(path, &mut hasher)?;
        }
        Ok(ArtifactDigest(format!("{:x}", hasher.finalize())))
    }
    fn profile(&self, ArtifactPath(directory): ArtifactPath<'_>) -> Result<Tool, Error> {
        super::tool::schemas::tool(&std::fs::read_to_string(
            directory.join(constants::MANIFEST),
        )?)
    }
    fn projects(
        &self,
        ArtifactPath(directory): ArtifactPath<'_>,
    ) -> Result<ryo_benchmark_contracts::domain::workloads::types::Suite, Error> {
        super::corpus::schemas::suite(&std::fs::read_to_string(
            directory.join(constants::MANIFEST),
        )?)
    }
    fn verify_tool(
        &self,
        ArtifactPath(executable): ArtifactPath<'_>,
        tool: &Tool,
    ) -> Result<(), Error> {
        verify_executable(executable, tool)
    }
}

/// Verify exact sources and evidence inside their separately explicit boundaries.
fn verify(artifacts: CaseArtifacts<'_>) -> Result<(), SuiteError> {
    if source_digest(artifacts.root, artifacts.files)?.0 != artifacts.source_sha256.0 {
        return Err(SuiteError::Invalid(
            constants::STALE_REFERENCE_DISCREPANCY_SOURCE_IDENTITY.into(),
        ));
    }
    if artifacts
        .file_metadata_files
        .iter()
        .any(|file| !artifacts.files.contains(file))
        || (artifacts.file_metadata == ArtifactMetadata::Undeclared
            && !artifacts.file_metadata_files.is_empty())
    {
        return Err(SuiteError::Invalid(
            constants::UNDECLARED_REFERENCE_DISCREPANCY_OBSERVATION.into(),
        ));
    }
    if let ArtifactMetadata::Declared(expected) = artifacts.file_metadata {
        let actual = super::file_metadata::schemas::actual_digest(
            artifacts.root,
            artifacts.file_metadata_files,
        )
        .map_err(|error| SuiteError::Invalid(error.to_string()))?;
        if actual != expected.0 {
            return Err(SuiteError::Invalid(
                constants::STALE_REFERENCE_DISCREPANCY_FILE_METADATA_ID.into(),
            ));
        }
    }
    let boundary = artifacts.profile.canonicalize()?;
    for evidence in artifacts.evidence {
        relative(&evidence.path)?;
        let path = boundary.join(&evidence.path).canonicalize()?;
        if !path.starts_with(&boundary) {
            return Err(SuiteError::Invalid(
                constants::STALE_OR_EXTERNAL_DISCREPANCY_EVIDENCE.into(),
            ));
        }
        let bytes = std::fs::read(path)?;
        if format!("{:x}", Sha256::digest(&bytes)) != evidence.sha256.0 {
            return Err(SuiteError::Invalid(
                constants::STALE_DISCREPANCY_EVIDENCE_BYTES.into(),
            ));
        }
        verify_requirement(&bytes, &evidence.requirement)?;
    }
    Ok(())
}

/// Read exact declared raw bytes under the profile boundary after validating their digests.
fn read(
    profile: &Path,
    artifacts: &[crate::outbound::suite::types::EvidenceArtifact],
) -> Result<std::collections::BTreeMap<std::path::PathBuf, ArtifactBytes>, SuiteError> {
    let boundary = profile.canonicalize()?;
    let mut result = std::collections::BTreeMap::new();
    for artifact in artifacts {
        relative(&artifact.path)?;
        let path = boundary.join(&artifact.path).canonicalize()?;
        if !path.starts_with(&boundary) || !path.is_file() {
            return Err(SuiteError::Invalid(
                constants::RAW_ARTIFACT_ESCAPES_ITS_PROFILE.into(),
            ));
        }
        let bytes = std::fs::read(path)?;
        if format!("{:x}", Sha256::digest(&bytes)) != artifact.sha256.0 {
            return Err(SuiteError::Invalid(
                constants::RAW_ARTIFACT_BYTE_IDENTITY_DIFFERS.into(),
            ));
        }
        if result
            .insert(artifact.path.clone(), ArtifactBytes(bytes))
            .is_some()
        {
            return Err(SuiteError::Invalid(
                constants::DUPLICATE_RAW_ARTIFACT_PATH.into(),
            ));
        }
    }
    Ok(result)
}

/// Canonical source framing shared by fresh observations and discrepancy evidence.
fn source_digest(root: &Path, files: &[std::path::PathBuf]) -> Result<SourceDigest, SuiteError> {
    let root = root.canonicalize()?;
    let mut files = files.to_vec();
    files.sort_by(|left, right| left.to_string_lossy().cmp(&right.to_string_lossy()));
    let mut source = Sha256::new();
    for file in files {
        relative(&file)?;
        let path = root.join(&file).canonicalize()?;
        if !path.starts_with(&root) {
            return Err(SuiteError::Invalid(
                constants::SOURCE_ARTIFACT_ESCAPES_ITS_BOUNDARY.into(),
            ));
        }
        source.update(
            file.to_str()
                .ok_or_else(|| SuiteError::Invalid(constants::NONUTF8_SOURCE_PATH.into()))?
                .as_bytes(),
        );
        source.update([0]);
        source.update(std::fs::read(path)?);
    }
    Ok(SourceDigest(format!("{:x}", source.finalize())))
}
