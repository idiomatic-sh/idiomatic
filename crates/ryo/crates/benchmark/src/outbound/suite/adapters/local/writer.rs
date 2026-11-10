//! The adapter's traversal and serialization of the suite artifacts it stages, observes and fingerprints.
use super::constants;
use crate::outbound::suite::errors::SuiteError;
use crate::outbound::suite::types::{
    CaseClaim, ClaimedEvidence, ClaimedProcess, MaterializedCase, MaterializedContext,
};
use ryo_benchmark_contracts::domain::artifacts::types::FileExecutable;
use ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath;
use ryo_benchmark_contracts::domain::documents::types::text::value::{CaseName, RuleName};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::{StagePurpose, Tool};
use sha2::{Digest as _, Sha256};
use std::collections::BTreeSet;
use std::path::{Component, Path};

/// The directory of one authored rule below the rules directory, refused when it would escape it.
pub(super) fn rule_directory(
    rules: &Path,
    rule: &ryo::linter::RuleId,
) -> Result<std::path::PathBuf, Error> {
    let name = rule.to_string();
    let relative = RelativeArtifactPath::try_from(Path::new(&name))?;
    Ok(rules.join(relative.as_ref()))
}

/// Refuse module paths that escape the profile or traverse symlinks.
pub(super) fn validate_query_modules(directory: &Path, tool: &Tool) -> Result<(), Error> {
    let root = directory.canonicalize()?;
    for (id, relative) in &tool.query_modules {
        let _module = ryo::query::QueryModuleSource::try_new(&id.0, constants::MODULE_SOURCE)
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

/// Freeze the shared project manifest and every declared patch in an owned directory.
#[expect(
    clippy::create_dir,
    reason = "snapshot staging must refuse an existing destination rather than merge artifacts"
)]
pub(super) fn stage_projects(source: &Path, destination: &Path) -> Result<(), Error> {
    let source = source.canonicalize()?;
    let bytes = std::fs::read(source.join(constants::MANIFEST))?;
    let manifest = super::corpus::schemas::suite(
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

/// Materialize only the explicitly authored subset in an owned empty directory.
#[expect(
    clippy::create_dir,
    reason = "snapshot staging must refuse an existing destination rather than merge artifacts"
)]
pub(super) fn stage(
    directory: &Path,
    destination: &Path,
    authored: &[ryo::linter::RuleId],
    definitions: &BTreeSet<ryo::linter::RuleId>,
    purpose: StagePurpose,
) -> Result<(), Error> {
    let tool = super::tool::schemas::tool(&std::fs::read_to_string(
        directory.join(constants::MANIFEST),
    )?)?;
    validate_query_modules(directory, &tool)?;
    for rule in authored.iter().chain(definitions.iter()) {
        let _path = rule_directory(&directory.join(constants::RULES), rule)?;
    }
    std::fs::create_dir(destination)?;
    let original = std::fs::read_to_string(directory.join(constants::MANIFEST))?;
    let manifest = if tool.defaults == authored {
        original
    } else {
        super::tool::schemas::narrowed(&original, authored, definitions)?
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
        if purpose == StagePurpose::Certification {
            copy_evidence(directory, destination, &target.join(constants::CASES))?;
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

/// Hash artifact names and contents with deterministic recursive framing.
pub(super) fn fingerprint_path(path: &Path, hasher: &mut Sha256) -> Result<(), Error> {
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

/// Read the authored reference certificates; interpretation belongs to application orchestration.
pub(super) fn case_claims(
    profile: &Path,
    owners: &[ryo::linter::RuleId],
) -> Result<Vec<CaseClaim>, SuiteError> {
    let mut result = Vec::new();
    for entry in std::fs::read_dir(profile.join(constants::RULES))? {
        let entry = entry?;
        if !entry.path().join(constants::QUERY).is_file() {
            continue;
        }
        let rule = entry
            .file_name()
            .into_string()
            .map_err(|_bytes| SuiteError::Invalid(constants::NON_UTF8_RULE_DIRECTORY.into()))?;
        let rule = super::rule_cases::schemas::rule(&rule)
            .map_err(|error| SuiteError::Invalid(error.to_string()))?;
        if !owners.contains(&rule) {
            continue;
        }
        let text = std::fs::read_to_string(entry.path().join(constants::CASES))?;
        for (name, discrepancy, basis) in super::rule_cases::schemas::case_claims(&text)? {
            result.push(CaseClaim {
                rule: RuleName(rule.to_string()),
                name: CaseName(name),
                basis,
                discrepancy,
            });
        }
    }
    Ok(result)
}

/// Snapshot only explicitly declared evidence, preserving exact bytes.
pub(super) fn copy_evidence(
    profile: &Path,
    destination: &Path,
    cases: &Path,
) -> std::io::Result<()> {
    let claims = super::rule_cases::schemas::case_claims(&std::fs::read_to_string(cases)?)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    let boundary = profile.canonicalize()?;
    for (_, claim, _) in claims {
        let mut artifacts = claim
            .evidence
            .iter()
            .map(|item| {
                let (ClaimedEvidence::LanguageSpec { path, sha256, .. }
                | ClaimedEvidence::IndependentLanguageRun { path, sha256, .. }
                | ClaimedEvidence::PublicToolContract { path, sha256, .. }
                | ClaimedEvidence::PriorWorkerRun { path, sha256, .. }) = item;
                (path.clone(), sha256.0.clone())
            })
            .collect::<Vec<_>>();
        if let ClaimedProcess::Failure(failure) = &claim.process {
            for observation in &failure.observations {
                for item in [
                    &observation.invocation,
                    &observation.stdout,
                    &observation.stderr,
                ] {
                    artifacts.push((item.path.clone(), item.sha256.0.clone()));
                }
            }
        }
        for (path, expected) in artifacts {
            relative(&path)
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
            let requested = boundary.join(&path);
            let contextual = |error: std::io::Error| {
                std::io::Error::new(
                    error.kind(),
                    format!("case evidence {}: {error}", requested.display()),
                )
            };
            let source = requested.canonicalize().map_err(contextual)?;
            let bytes = std::fs::read(&source).map_err(contextual)?;
            if !source.starts_with(&boundary) || format!("{:x}", Sha256::digest(&bytes)) != expected
            {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    constants::STALE_OR_EXTERNAL_DISCREPANCY_EVIDENCE,
                ));
            }
            let target = destination.join(&path);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(target, bytes)?;
        }
    }
    Ok(())
}

/// Write only selected source facts; semantic preparation and expected reports stay above this port.
pub(super) fn materialize(
    boundary: &Path,
    sources: &ryo_benchmark_contracts::domain::workloads::types::Case,
    context: &MaterializedContext,
) -> Result<MaterializedCase, Error> {
    let root = match context {
        MaterializedContext::Isolated => boundary.to_owned(),
        MaterializedContext::Sealed(manifest) => manifest.boundary.join(&manifest.logical_root),
    };
    std::fs::create_dir_all(&root)?;
    for (file, source) in &sources.files {
        let relative = RelativeArtifactPath::try_from(Path::new(file.0.as_ref()))?;
        let path = root.join(relative.as_ref());
        std::fs::create_dir_all(
            path.parent()
                .ok_or_else(|| Error::Invalid(super::constants::MISSING_CASE_PARENT.into()))?,
        )?;
        std::fs::write(&path, source.0.as_bytes())?;
        if let ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration::Declared(
            values,
        ) = &sources.file_metadata
        {
            if let Some(executable) = values.get(file) {
                super::file_metadata::schemas::stage_executable(
                    &path,
                    *executable == FileExecutable::Set,
                )?;
            }
        }
    }
    Ok(MaterializedCase { root })
}

/// Materialize only the declared Given entries and reject contradictions with written sources.
pub(super) fn materialize_inventory(
    root: &Path,
    inventory: &ryo::linter::GivenProjectInventory,
) -> Result<(), Error> {
    for entry in &inventory.entries {
        let relative = RelativeArtifactPath::try_from(Path::new(entry.path.0.as_ref()))?;
        let path = root.join(relative.as_ref());
        match entry.state {
            ryo::linter::ProjectEntryState::File if !path.is_file() => {
                return Err(Error::Invalid(
                    constants::GIVEN_PROJECT_FILE_ENTRY_LACKS_STAGED.into(),
                ));
            }
            ryo::linter::ProjectEntryState::Directory => {
                if path.is_file() {
                    return Err(Error::Invalid(
                        constants::GIVEN_PROJECT_DIRECTORY_COLLIDES_WITH_A.into(),
                    ));
                }
                std::fs::create_dir_all(&path)?;
            }
            ryo::linter::ProjectEntryState::Missing if std::fs::metadata(&path).is_ok() => {
                return Err(Error::Invalid(
                    constants::GIVEN_PROJECT_MISSING_ENTRY_COLLIDES_WITH.into(),
                ));
            }
            ryo::linter::ProjectEntryState::Other => {
                return Err(Error::Invalid(
                    constants::GIVEN_PROJECT_NON_FILE_ENTRY_REQUIRES.into(),
                ));
            }
            ryo::linter::ProjectEntryState::File
            | ryo::linter::ProjectEntryState::Missing
            | ryo::linter::ProjectEntryState::Unknown => {}
        }
    }
    Ok(())
}

/// Refuse platform-specific escape spellings before touching artifact paths.
pub(super) fn relative(path: &Path) -> Result<(), SuiteError> {
    let text = path
        .to_str()
        .ok_or_else(|| SuiteError::Invalid(constants::NON_UTF8_ARTIFACT_PATH.into()))?;
    if text.is_empty()
        || text.contains(['\\', ':'])
        || !path
            .components()
            .all(|item| matches!(item, Component::Normal(_)))
    {
        return Err(SuiteError::Invalid(
            constants::UNSAFE_ARTIFACT_RELATIVE_PATH.into(),
        ));
    }
    Ok(())
}
