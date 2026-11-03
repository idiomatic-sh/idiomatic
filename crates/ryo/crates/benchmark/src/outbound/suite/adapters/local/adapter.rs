//! Validate pins, cases and rule coverage before running contestants.
use super::{constants, discrepancy_artifacts};
use crate::domain::diagnostics::errors::DiagnosticsError as Error;
use crate::domain::suite::service::{relative_path, validate_case_sources, validate_cases};
use crate::domain::suite::types::ArtifactDigest;
use crate::domain::suite::types::audit::value::Audit;
use crate::domain::suite::types::{Cases, Tool};
use crate::outbound::suite::interface::SuitePort;
use crate::outbound::suite::types::{ArtifactPath, ArtifactSet};
use sha2::{Digest as _, Sha256};
use std::collections::BTreeSet;
use std::path::Path;

/// Validate default dispositions and require independent sources for every authored query.
fn audit(directory: &Path, tool: &Tool) -> Result<Audit, Error> {
    validate_query_modules(directory, &tool.query_modules)?;
    let defaults: BTreeSet<_> = tool.defaults.iter().cloned().collect();
    if defaults.is_empty() || defaults.len() != tool.defaults.len() {
        return Err(Error::Invalid(constants::INVALID_DEFAULTS.into()));
    }
    for (rule, issue) in tool.deferred.iter().chain(&tool.pending) {
        if !defaults.contains(rule)
            || issue.0.is_empty()
            || (tool.deferred.contains_key(rule) && tool.pending.contains_key(rule))
        {
            return Err(Error::Invalid(format!(
                "invalid or overlapping default disposition: {rule} -> {issue}"
            )));
        }
    }
    let mut implemented = BTreeSet::new();
    for entry in std::fs::read_dir(directory.join(constants::RULES))? {
        let entry = entry?;
        let rule = entry
            .file_name()
            .into_string()
            .map_err(|_invalid_utf8| Error::Invalid(constants::INVALID_RULE_ID.into()))?;
        let rule = super::schemas::rule(&rule)?;
        if !defaults.contains(&rule) {
            return Err(Error::Invalid(format!(
                "rule {rule} is not a shipped default"
            )));
        }
        let query = match std::fs::read_to_string(entry.path().join(constants::QUERY)) {
            Ok(query) => Some(query),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        if query.as_ref().is_some_and(|query| query.trim().is_empty()) {
            return Err(Error::Invalid(format!("empty query for {rule}")));
        }
        let cases: Cases = super::schemas::cases(&std::fs::read_to_string(
            entry.path().join(constants::CASES),
        )?)?;
        validate_case_sources(&cases)?;
        if query.is_some() {
            validate_cases(&rule, &cases)?;
            if tool.deferred.contains_key(&rule) || tool.pending.contains_key(&rule) {
                return Err(Error::Invalid(format!(
                    "authored rule still has a deferred or pending disposition: {rule}"
                )));
            }
            let _already_present = implemented.insert(rule);
        }
    }
    Ok(Audit {
        missing: defaults.difference(&implemented).cloned().collect(),
        implemented: implemented.into_iter().collect(),
    })
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
        relative_path(relative)?;
        let mut current = root.clone();
        for component in relative.components() {
            current.push(component);
            let metadata = std::fs::symlink_metadata(&current)?;
            if metadata.file_type().is_symlink() {
                return Err(Error::Invalid(format!(
                    "query module is a symlink: {}",
                    relative.display()
                )));
            }
        }
        let canonical = current.canonicalize()?;
        if !canonical.starts_with(&root) || !canonical.is_file() {
            return Err(Error::Invalid(format!(
                "query module is outside profile or not a file: {}",
                relative.display()
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
pub(crate) struct SuiteLocalAdapter;
impl SuitePort for SuiteLocalAdapter {
    fn validate_file_metadata(
        &self,
        request: &crate::domain::suite::types::RunRequest,
    ) -> Result<(), Error> {
        super::input_metadata::validate(request)
    }

    fn case_receipt_file(&self) -> Option<std::path::PathBuf> {
        std::env::var_os("RYO_VERIFY_CASE_RESULTS").map(std::path::PathBuf::from)
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
    ) -> Result<
        Vec<crate::outbound::suite::types::discrepancy::CaseReferenceWire>,
        crate::outbound::suite::types::discrepancy::ArtifactError,
    > {
        super::discrepancy_artifacts::case_reference_wires(profile)
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
    fn cases(
        &self,
        ArtifactPath(rules): ArtifactPath<'_>,
        ArtifactPath(scratch): ArtifactPath<'_>,
    ) -> Result<Vec<crate::domain::suite::types::PreparedCase>, Error> {
        prepare_cases(rules, scratch)
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
            relative_path(&project.patch)?;
            let target = destination.join(&project.patch);
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
        audit: &Audit,
    ) -> Result<(), Error> {
        let tool = super::schemas::tool(&std::fs::read_to_string(
            directory.join(constants::MANIFEST),
        )?)?;
        validate_query_modules(directory, &tool.query_modules)?;
        std::fs::create_dir(destination)?;
        std::fs::write(
            destination.join(constants::MANIFEST),
            std::fs::read(directory.join(constants::MANIFEST))?,
        )?;
        std::fs::create_dir(destination.join(constants::RULES))?;
        for relative in tool.query_modules.values() {
            let target = destination.join(relative);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let source = directory.join(relative);
            std::fs::write(target, std::fs::read(source)?)?;
        }
        for rule in &audit.implemented {
            relative_path(rule.to_string())?;
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
            super::discrepancy_artifacts::copy_evidence(
                directory,
                destination,
                &target.join(constants::CASES),
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
    ) -> Result<crate::domain::suite::types::Suite, Error> {
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

    fn audit(
        &self,
        ArtifactPath(directory): ArtifactPath<'_>,
        tool: &Tool,
    ) -> Result<Audit, Error> {
        audit(directory, tool)
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

use crate::domain::diagnostics::types::RunReport;
/// Materialize validated independent sources into isolated per-case directories.
fn prepare_cases(
    rules: &Path,
    scratch: &Path,
) -> Result<Vec<crate::domain::suite::types::PreparedCase>, Error> {
    use crate::domain::suite::types::{PreparedCase, RunRequest};
    let mut prepared = Vec::new();
    let profile = rules
        .parent()
        .ok_or_else(|| Error::Invalid("case rules have no profile parent".into()))?
        .join(constants::MANIFEST);
    let profile = match std::fs::read_to_string(profile) {
        Ok(text) => Some(super::schemas::tool(&text)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    let context_mode = profile.as_ref().map_or(
        crate::domain::suite::types::ProjectContextMode::Isolated,
        |tool| tool.project_context_mode,
    );
    for entry in std::fs::read_dir(rules)? {
        let entry = entry?;
        let rule = entry
            .file_name()
            .into_string()
            .map_err(|_invalid_utf8| Error::Invalid(constants::INVALID_RULE_ID_TEXT.into()))?;
        let rule = super::schemas::rule(&rule)?;
        let cases: Cases = super::schemas::cases(&std::fs::read_to_string(
            entry.path().join(constants::CASES),
        )?)?;
        validate_case_sources(&cases)?;
        if profile
            .as_ref()
            .is_some_and(|tool| tool.file_observation.is_some())
            && cases.cases.iter().any(|case| {
                case.file_metadata
                    .as_ref()
                    .is_none_or(|values| values.len() != case.files.len())
            })
        {
            return Err(Error::Invalid(
                "file-observation-required profile needs explicit metadata for every source".into(),
            ));
        }
        match std::fs::read_to_string(entry.path().join(constants::QUERY)) {
            Ok(query) if !query.trim().is_empty() => {}
            Ok(_) => return Err(Error::Invalid(format!("empty query for {rule}"))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        }
        validate_cases(&rule, &cases)?;
        for (index, case) in cases.cases.into_iter().enumerate() {
            let boundary = scratch.join(rule.to_string()).join(index.to_string());
            let context_manifest = match (context_mode, &case.context) {
                (crate::domain::suite::types::ProjectContextMode::Isolated, None) => None,
                (crate::domain::suite::types::ProjectContextMode::ListedDisk, Some(context)) => {
                    Some(
                        crate::outbound::context::adapters::local::adapter::materialize(
                            &boundary,
                            context,
                            &case.files,
                        )?,
                    )
                }
                _ => {
                    return Err(Error::Invalid(
                        "case context must agree with explicit profile context mode".into(),
                    ));
                }
            };
            let root = context_manifest.as_ref().map_or(boundary, |manifest| {
                manifest.boundary.join(&manifest.logical_root)
            });
            std::fs::create_dir_all(&root)?;
            for (file, source) in &case.files {
                relative_path(file.0.as_ref())?;
                let path = root.join(file.0.as_ref());
                std::fs::create_dir_all(
                    path.parent()
                        .ok_or_else(|| Error::Invalid(constants::MISSING_CASE_PARENT.into()))?,
                )?;
                std::fs::write(&path, source.0.as_bytes())?;
                if let Some(executable) = case
                    .file_metadata
                    .as_ref()
                    .and_then(|values| values.get(file))
                {
                    super::input_metadata::stage(&path, *executable)?;
                }
            }
            let request = RunRequest {
                file_metadata: case.file_metadata,
                context_manifest,
                root,
                files: case.files.keys().cloned().collect(),
                state: scratch
                    .join(constants::CACHE)
                    .join(rule.to_string())
                    .join(index.to_string()),
            };
            let expected = expected_report(&rule, request.files.clone(), case.expected);
            prepared.push(PreparedCase {
                reference_discrepancy: None,
                comparison: case.comparison,
                rule: rule.clone(),
                name: case.name,
                request,
                expected,
            });
        }
    }
    if prepared.is_empty() {
        return Err(Error::Invalid(constants::NO_EXECUTABLE_CASES.into()));
    }
    Ok(prepared)
}

/// Preserve each authored diagnostic identity and sort the independent expected multiset.
fn expected_report(
    rule: &ryo::linter::RuleId,
    files: Vec<ryo::linter::RelativePath>,
    expected: Vec<crate::domain::suite::types::Expected>,
) -> RunReport {
    let mut report = RunReport {
        execution_metadata: crate::domain::diagnostics::types::ExecutionMetadata::Unknown,
        project_context: None,
        files,
        skipped: Vec::new(),
        coverage: None,
        diagnostics: expected
            .into_iter()
            .map(
                |finding| crate::domain::diagnostics::types::BenchmarkFinding {
                    rule: match finding.identity {
                        crate::domain::suite::types::ExpectedRule::Parent => rule.clone(),
                        crate::domain::suite::types::ExpectedRule::Explicit(value) => value,
                    },
                    file: finding.file,
                    start: finding.start,
                    end: finding.end,
                },
            )
            .collect(),
    };
    report.diagnostics.sort();
    report
}
