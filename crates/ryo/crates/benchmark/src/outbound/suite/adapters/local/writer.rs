//! The adapter's traversal and serialization of the suite artifacts it stages and fingerprints.
use super::constants;
use ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::{StagePurpose, Tool};
use sha2::{Digest as _, Sha256};
use std::collections::BTreeSet;
use std::path::Path;

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
        let _module = ryo::query::QueryModuleSource::try_new(id, constants::MODULE_SOURCE)
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
