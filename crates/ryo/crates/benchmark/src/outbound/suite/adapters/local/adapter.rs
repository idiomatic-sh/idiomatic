//! Validate pins, cases and rule coverage before running contestants.
use super::{constants, discrepancy_artifacts};
use crate::outbound::suite::interface::SuitePort;
use crate::outbound::suite::types::{ArtifactPath, ArtifactSet};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::Cases;
use ryo_benchmark_contracts::domain::workloads::types::Tool;
use sha2::{Digest as _, Sha256};
use std::collections::BTreeSet;
use std::path::Path;

fn rule_directory(rules: &Path, rule: &ryo::linter::RuleId) -> Result<std::path::PathBuf, Error> {
    let name = rule.to_string();
    let relative = RelativeArtifactPath::try_from(Path::new(&name))?;
    Ok(rules.join(relative.as_ref()))
}

/// Refuse module paths that escape the profile or traverse symlinks.
fn validate_query_modules(
    directory: &Path,
    modules: &std::collections::BTreeMap<String, std::path::PathBuf>,
) -> Result<(), Error> {
    let root = directory.canonicalize()?;
    for (id, relative) in modules {
        let _module = ryo::query::QueryModuleSource::try_new(id, "module source")
            .map_err(|error| Error::Invalid(error.to_string()))?;
        let relative = RelativeArtifactPath::try_from(relative.as_path())?;
        let mut current = root.clone();
        for component in relative.as_ref().components() {
            current.push(component);
            let metadata = std::fs::symlink_metadata(&current)?;
            if metadata.file_type().is_symlink() {
                return Err(Error::Invalid(format!(
                    "query module is a symlink: {}",
                    relative.as_ref().display()
                )));
            }
        }
        let canonical = current.canonicalize()?;
        if !canonical.starts_with(&root) || !canonical.is_file() {
            return Err(Error::Invalid(format!(
                "query module is outside profile or not a file: {}",
                relative.as_ref().display()
            )));
        }
        let _module_text = std::fs::read_to_string(canonical)?;
    }
    Ok(())
}

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
    /// Bind an optional receipt destination chosen explicitly by executable composition.
    pub(crate) fn new(case_receipts: Option<std::path::PathBuf>) -> Self {
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
        files: &[std::path::PathBuf],
    ) -> Result<String, crate::outbound::suite::types::discrepancy::ArtifactError> {
        discrepancy_artifacts::source_digest(root.0, files)
    }

    fn read_discrepancy_artifacts(
        &self,
        profile: ArtifactPath<'_>,
        artifacts: &[crate::outbound::suite::types::discrepancy::EvidenceArtifact],
    ) -> Result<
        std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>,
        crate::outbound::suite::types::discrepancy::ArtifactError,
    > {
        discrepancy_artifacts::read(profile.0, artifacts)
    }
    fn record_case(
        &self,
        receipt: ArtifactPath<'_>,
        record: serde_json::Value,
        process: Option<crate::outbound::suite::types::process::CapturedProcess<'_>>,
    ) -> Result<(), crate::outbound::suite::types::discrepancy::ArtifactError> {
        super::case_observations::record(receipt.0, record, process)
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
                super::schemas::rule(&name)
            })
            .collect()
    }
    fn rule_query(
        &self,
        ArtifactPath(profile): ArtifactPath<'_>,
        rule: &ryo::linter::RuleId,
    ) -> Result<Option<String>, Error> {
        match std::fs::read_to_string(rule_directory(profile, rule)?.join(constants::QUERY)) {
            Ok(query) => Ok(Some(query)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
    fn rule_cases(
        &self,
        ArtifactPath(profile): ArtifactPath<'_>,
        rule: &ryo::linter::RuleId,
    ) -> Result<Cases, Error> {
        super::schemas::cases(&std::fs::read_to_string(
            rule_directory(profile, rule)?.join(constants::CASES),
        )?)
    }
    fn verify_query_modules(
        &self,
        ArtifactPath(profile): ArtifactPath<'_>,
        tool: &Tool,
    ) -> Result<(), Error> {
        validate_query_modules(profile, &tool.query_modules)
    }
    fn materialize_case(
        &self,
        ArtifactPath(destination): ArtifactPath<'_>,
        case: &ryo_benchmark_contracts::domain::workloads::types::Case,
    ) -> Result<crate::outbound::suite::types::MaterializedCase, Error> {
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
        tracked: &[std::path::PathBuf],
    ) -> Result<ryo_benchmark_contracts::domain::context::types::ContextManifest, Error> {
        crate::outbound::context::adapters::local::adapter::seal_existing(root, sources, tracked)
    }
    #[expect(
        clippy::create_dir,
        reason = "snapshot staging must refuse an existing destination rather than merge artifacts"
    )]
    fn stage_projects(
        &self,
        ArtifactPath(source): ArtifactPath<'_>,
        ArtifactPath(destination): ArtifactPath<'_>,
    ) -> Result<(), Error> {
        let source = source.canonicalize()?;
        let bytes = std::fs::read(source.join(constants::MANIFEST))?;
        let manifest = super::schemas::suite(
            std::str::from_utf8(&bytes).map_err(|error| Error::Invalid(error.to_string()))?,
        )?;
        std::fs::create_dir(destination)?;
        std::fs::write(destination.join(constants::MANIFEST), bytes)?;
        for project in manifest.projects {
            let relative = RelativeArtifactPath::try_from(project.patch.as_path())?;
            let target = destination.join(relative.as_ref());
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let patch = source.join(project.patch).canonicalize()?;
            if !patch.starts_with(&source) {
                return Err(Error::Invalid(constants::PATCH_ESCAPE.into()));
            }
            std::fs::write(target, std::fs::read(patch)?)?;
        }
        Ok(())
    }
    #[expect(
        clippy::create_dir,
        reason = "snapshot staging must refuse an existing destination rather than merge artifacts"
    )]
    fn stage(
        &self,
        ArtifactPath(directory): ArtifactPath<'_>,
        ArtifactPath(destination): ArtifactPath<'_>,
        authored: &[ryo::linter::RuleId],
        definitions: &BTreeSet<ryo::linter::RuleId>,
        purpose: ryo_benchmark_contracts::domain::workloads::types::StagePurpose,
    ) -> Result<(), Error> {
        let tool = super::schemas::tool(&std::fs::read_to_string(
            directory.join(constants::MANIFEST),
        )?)?;
        validate_query_modules(directory, &tool.query_modules)?;
        for rule in authored.iter().chain(definitions.iter()) {
            let _path = rule_directory(&directory.join(constants::RULES), rule)?;
        }
        std::fs::create_dir(destination)?;
        let original = std::fs::read_to_string(directory.join(constants::MANIFEST))?;
        let narrowed = tool.defaults != authored;
        let manifest = if narrowed {
            let mut value: toml::Value = toml::from_str(&original)?;
            let table = value
                .as_table_mut()
                .ok_or_else(|| Error::Invalid("profile must be a table".into()))?;
            table.insert(
                "defaults".into(),
                toml::Value::Array(
                    authored
                        .iter()
                        .map(|id| toml::Value::String(id.to_string()))
                        .collect(),
                ),
            );
            table.insert(
                "supporting-rules".into(),
                toml::Value::Array(
                    definitions
                        .iter()
                        .filter(|id| !authored.contains(id))
                        .map(|id| toml::Value::String(id.to_string()))
                        .collect(),
                ),
            );
            table.remove("pending");
            table.remove("deferred");
            for key in ["rule-observations", "rule-applicability"] {
                if let Some(entries) = table.get_mut(key).and_then(toml::Value::as_table_mut) {
                    entries.retain(|id, _| definitions.iter().any(|rule| rule.as_ref() == id));
                }
            }
            toml::to_string(&value).map_err(|error| Error::Invalid(error.to_string()))?
        } else {
            original
        };
        std::fs::write(destination.join(constants::MANIFEST), manifest)?;
        std::fs::create_dir(destination.join(constants::RULES))?;
        for relative in tool.query_modules.values() {
            let target = destination.join(relative);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let source = directory.join(relative);
            std::fs::write(target, std::fs::read(source)?)?;
        }
        for rule in authored {
            let rule_path = rule.to_string();
            let _relative = RelativeArtifactPath::try_from(Path::new(&rule_path))?;
            let target = destination.join(constants::RULES).join(rule.to_string());
            std::fs::create_dir(&target)?;
            let bytes = std::fs::read(
                directory
                    .join(constants::RULES)
                    .join(rule.to_string())
                    .join(constants::QUERY),
            )?;
            std::fs::write(target.join(constants::QUERY), bytes)?;
            std::fs::write(
                target.join(constants::CASES),
                std::fs::read(
                    directory
                        .join(constants::RULES)
                        .join(rule.to_string())
                        .join(constants::CASES),
                )?,
            )?;
            if purpose
                == ryo_benchmark_contracts::domain::workloads::types::StagePurpose::Certification
            {
                super::discrepancy_artifacts::copy_evidence(
                    directory,
                    destination,
                    &target.join(constants::CASES),
                )?;
            }
        }
        for rule in definitions.iter().filter(|id| !authored.contains(id)) {
            let target = destination.join(constants::RULES).join(rule.to_string());
            std::fs::create_dir(&target)?;
            std::fs::write(
                target.join(constants::QUERY),
                std::fs::read(
                    directory
                        .join(constants::RULES)
                        .join(rule.to_string())
                        .join(constants::QUERY),
                )?,
            )?;
        }
        Ok(())
    }
    fn fingerprint(&self, ArtifactSet(paths): ArtifactSet<'_>) -> Result<ArtifactDigest, Error> {
        let mut hasher = Sha256::new();
        for path in paths {
            fingerprint_path(path, &mut hasher)?;
        }
        Ok(ArtifactDigest(format!("{:x}", hasher.finalize())))
    }
    fn profile(&self, ArtifactPath(directory): ArtifactPath<'_>) -> Result<Tool, Error> {
        super::schemas::tool(&std::fs::read_to_string(
            directory.join(constants::MANIFEST),
        )?)
    }
    fn projects(
        &self,
        ArtifactPath(directory): ArtifactPath<'_>,
    ) -> Result<ryo_benchmark_contracts::domain::workloads::types::Suite, Error> {
        super::schemas::suite(&std::fs::read_to_string(
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

/// Hash artifact names and contents with deterministic recursive framing.
fn fingerprint_path(path: &Path, hasher: &mut Sha256) -> Result<(), Error> {
    let metadata = std::fs::symlink_metadata(path)?;
    let name = path
        .to_str()
        .ok_or_else(|| Error::Invalid(constants::INVALID_EVIDENCE_PATH.into()))?;
    hasher.update(name.len().to_le_bytes());
    hasher.update(name.as_bytes());
    if metadata.is_file() {
        let bytes = std::fs::read(path)?;
        hasher.update(bytes.len().to_le_bytes());
        hasher.update(bytes);
    } else if metadata.is_dir() {
        let mut children = std::fs::read_dir(path)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()?;
        children.sort();
        for child in children {
            fingerprint_path(&child, hasher)?;
        }
    } else {
        return Err(Error::Invalid(constants::INVALID_EVIDENCE_KIND.into()));
    }
    Ok(())
}
