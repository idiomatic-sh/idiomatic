//! Validate pins, cases and rule coverage before running contestants.
use super::writer::{fingerprint_path, rule_directory, validate_query_modules};
use super::{constants, discrepancy_artifacts};
use crate::outbound::suite::interface::SuitePort;
use crate::outbound::suite::types::{
    ArtifactBytes, ArtifactPath, ArtifactSet, ArtifactSize, ArtifactText, CaseReceipts,
    MaterializedCase, RuleQuery, ScratchDirectory, SelectedName,
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

#[derive(Debug)]
/// Reads and freezes validated benchmark artifacts from the local filesystem.
pub(crate) struct SuiteLocalAdapter {
    case_receipts: Option<std::path::PathBuf>,
}
impl SuiteLocalAdapter {
    /// Bind the receipt destination chosen explicitly by executable composition.
    pub(crate) fn new(case_receipts: CaseReceipts) -> Self {
        let case_receipts = match case_receipts {
            CaseReceipts::Unrecorded => None,
            CaseReceipts::Recorded(path) => Some(path),
        };
        Self { case_receipts }
    }
}
impl SuitePort for SuiteLocalAdapter {
    fn validate_file_metadata(
        &self,
        request: &ryo_benchmark_contracts::domain::requests::types::RunRequest,
    ) -> Result<(), Error> {
        super::input_metadata::validate(request)
    }

    fn case_receipt_file(&self) -> Option<std::path::PathBuf> {
        self.case_receipts.clone()
    }

    fn case_source_digest(
        &self,
        root: ArtifactPath<'_>,
        files: ArtifactSet<'_>,
    ) -> Result<SourceDigest, crate::outbound::suite::types::discrepancy::ArtifactError> {
        discrepancy_artifacts::source_digest(root.0, files.0).map(SourceDigest)
    }

    fn read_discrepancy_artifacts(
        &self,
        profile: ArtifactPath<'_>,
        artifacts: &[crate::outbound::suite::types::discrepancy::EvidenceArtifact],
    ) -> Result<
        std::collections::BTreeMap<std::path::PathBuf, ArtifactBytes>,
        crate::outbound::suite::types::discrepancy::ArtifactError,
    > {
        discrepancy_artifacts::read(profile.0, artifacts).map(|bytes| {
            bytes
                .into_iter()
                .map(|(path, bytes)| (path, ArtifactBytes(bytes)))
                .collect()
        })
    }
    fn record_case(
        &self,
        receipt: ArtifactPath<'_>,
        record: ryo_benchmark_contracts::domain::documents::types::observation_document::ObservationDocument,
        process: Option<crate::outbound::suite::types::process::CapturedProcess<'_>>,
    ) -> Result<(), crate::outbound::suite::types::discrepancy::ArtifactError> {
        super::case_observations::record(receipt.0, record, process)
    }
    fn case_observations(
        &self,
        receipt: ArtifactPath<'_>,
    ) -> Result<
        Vec<ryo_benchmark_contracts::domain::documents::types::observation_document::ObservationDocument>,
        crate::outbound::suite::types::discrepancy::ArtifactError,
    > {
        super::case_observations::read(receipt.0)
    }

    fn case_reference_wires(
        &self,
        ArtifactPath(profile): ArtifactPath<'_>,
        owners: &[ryo::linter::RuleId],
    ) -> Result<
        Vec<crate::outbound::suite::types::discrepancy::CaseReferenceWire>,
        crate::outbound::suite::types::discrepancy::ArtifactError,
    > {
        super::discrepancy_artifacts::case_reference_wires(profile, owners)
    }
    fn verify_discrepancy_artifacts(
        &self,
        artifacts: crate::outbound::suite::types::discrepancy::CaseArtifacts<'_>,
    ) -> Result<(), crate::outbound::suite::types::discrepancy::ArtifactError> {
        super::discrepancy_artifacts::verify(artifacts)
    }
    fn scratch(
        &self,
        ArtifactPath(parent): ArtifactPath<'_>,
        ArtifactText(prefix): ArtifactText<'_>,
    ) -> Result<ScratchDirectory, Error> {
        let mut builder = tempfile::Builder::new();
        let _prefixed = builder.prefix(prefix);
        let directory = if parent.as_os_str().is_empty() {
            builder.tempdir()?
        } else {
            builder.tempdir_in(parent)?
        };
        Ok(ScratchDirectory(directory.keep()))
    }
    fn discard(&self, ArtifactPath(directory): ArtifactPath<'_>) -> Result<(), Error> {
        std::fs::remove_dir_all(directory).map_err(Into::into)
    }
    fn copy_file(&self, ArtifactPath(from): ArtifactPath<'_>, ArtifactPath(to): ArtifactPath<'_>) -> Result<(), Error> {
        std::fs::copy(from, to).map(|_bytes| ()).map_err(Into::into)
    }
    fn ensure_directory(&self, ArtifactPath(directory): ArtifactPath<'_>) -> Result<std::path::PathBuf, Error> {
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
    fn selection(&self, ArtifactPath(file): ArtifactPath<'_>) -> Result<Vec<SelectedName>, Error> {
        super::selection::schemas::names(&std::fs::read(file)?)
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
    ) -> Result<MaterializedCase, Error> {
        super::input_metadata::materialize(destination, case)
    }
    fn materialize_inventory(
        &self,
        ArtifactPath(root): ArtifactPath<'_>,
        inventory: &ryo::linter::GivenProjectInventory,
    ) -> Result<(), Error> {
        super::input_metadata::materialize_inventory(root, inventory)
    }
    fn seal_context(
        &self,
        ArtifactPath(root): ArtifactPath<'_>,
        sources: &[ryo::linter::RelativePath],
        ArtifactSet(tracked): ArtifactSet<'_>,
    ) -> Result<ryo_benchmark_contracts::domain::context::types::ContextManifest, Error> {
        crate::outbound::context::adapters::local::adapter::seal_existing(root, sources, tracked)
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

