//! Materialize and validate explicitly sealed project context at the filesystem boundary.
use super::constants;
use super::writer::{PathScope, frame, no_symlinks, relative, resolve, walked_dependency};
use crate::outbound::context::interface::ContextPort;
use crate::outbound::context::types::CorpusFiles;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::context::constants::{AUXILIARY, MISSING, SOURCE};
use ryo_benchmark_contracts::domain::context::types::CaseContext;
use ryo_benchmark_contracts::domain::context::types::ContextBytes;
use ryo_benchmark_contracts::domain::context::types::ContextManifest;
use ryo_benchmark_contracts::domain::context::types::ProjectContextReceipt;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use sha2::{Digest as _, Sha256};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

/// Raw artifact identity, independent of the engine's opaque dependency fingerprint.
pub(crate) fn artifact(bytes: &ContextBytes) -> ArtifactDigest {
    ArtifactDigest(format!("{:x}", Sha256::digest(&bytes.0)))
}

/// Content seal includes declared absences and stable logical location, never temporary physical directories.
pub(crate) fn fingerprint(manifest: &ContextManifest) -> ArtifactDigest {
    let mut hash = Sha256::new();
    frame(&mut hash, OsStr::new(constants::LISTED_DISK_CONTEXT_DIGEST_TAG));
    frame(&mut hash, manifest.logical_root.as_os_str());
    for (kind, files) in [
        (SOURCE, &manifest.source_files),
        (AUXILIARY, &manifest.auxiliary_files),
    ] {
        frame(&mut hash, OsStr::new(kind));
        for (path, digest) in files {
            frame(&mut hash, path.as_os_str());
            frame(&mut hash, OsStr::new(&digest.0));
        }
    }
    let mut missing = manifest.missing.clone();
    missing.sort();
    for path in missing {
        frame(&mut hash, OsStr::new(MISSING));
        frame(&mut hash, path.as_os_str());
    }
    ArtifactDigest(format!("{:x}", hash.finalize()))
}

/// Validate found and missing dependencies without treating unreadable files as missing.
pub(crate) fn validate(manifest: &ContextManifest, actual_root: &Path) -> Result<(), Error> {
    relative(&manifest.logical_root, PathScope::Root)?;
    if !manifest.boundary.is_absolute()
        || actual_root != manifest.boundary.join(&manifest.logical_root)
    {
        return Err(Error::Invalid(
            constants::CONTEXT_LOGICAL_AND_PHYSICAL_ROOTS_DISAGREE.into(),
        ));
    }
    let mut missing = manifest.missing.clone();
    missing.sort();
    if missing
        .windows(2)
        .any(|pair| matches!(pair, [first, second] if first == second))
    {
        return Err(Error::Invalid(
            constants::DUPLICATE_MISSING_CONTEXT_PATH.into(),
        ));
    }
    for (path, expected) in manifest
        .source_files
        .iter()
        .chain(&manifest.auxiliary_files)
    {
        relative(path, PathScope::Member)?;
        no_symlinks(&manifest.boundary, path)?;
        if manifest.source_files.contains_key(path) && manifest.auxiliary_files.contains_key(path)
            || missing.contains(path)
        {
            return Err(Error::Invalid(
                constants::CONTEXT_PATH_HAS_CONFLICTING_ROLES.into(),
            ));
        }
        let bytes = ContextBytes(std::fs::read(manifest.boundary.join(path))?);
        if artifact(&bytes) != *expected {
            return Err(Error::Invalid(format!(
                "context bytes changed: {}",
                path.display()
            )));
        }
    }
    for path in &missing {
        relative(path, PathScope::Member)?;
        no_symlinks(&manifest.boundary, path)?;
        match std::fs::symlink_metadata(manifest.boundary.join(path)) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
            Ok(_) => {
                return Err(Error::Invalid(format!(
                    "missing context path appeared: {}",
                    path.display()
                )));
            }
        }
    }
    Ok(())
}

/// Seal actual existing corpus bytes without changing its filesystem or fabricating observations.
pub(crate) fn seal_existing(
    root: &Path,
    sources: &[ryo::linter::RelativePath],
    tracked: &[PathBuf],
) -> Result<ContextManifest, Error> {
    let boundary = std::path::absolute(root)?;
    let source_set: std::collections::BTreeSet<_> = sources
        .iter()
        .map(|p| PathBuf::from(p.0.as_ref()))
        .collect();
    let tracked_set: std::collections::BTreeSet<_> = tracked.iter().cloned().collect();
    if source_set.len() != sources.len()
        || tracked_set.len() != tracked.len()
        || !source_set.is_subset(&tracked_set)
    {
        return Err(Error::Invalid(
            constants::SEALED_CORPUS_REQUIRES_UNIQUE_ACTUAL_TRACKED.into(),
        ));
    }
    let mut manifest = ContextManifest {
        boundary,
        logical_root: PathBuf::from(Component::CurDir.as_os_str()),
        source_files: BTreeMap::new(),
        auxiliary_files: BTreeMap::new(),
        missing: Vec::new(),
    };
    for path in tracked {
        relative(path, PathScope::Member)?;
        no_symlinks(&manifest.boundary, path)?;
        let actual = manifest.boundary.join(path);
        let bytes = ContextBytes(std::fs::read(&actual).map_err(|error| {
            Error::Io(std::io::Error::new(
                error.kind(),
                format!("seal corpus {}: {error}", actual.display()),
            ))
        })?);
        let destination = if source_set.contains(path) {
            &mut manifest.source_files
        } else {
            &mut manifest.auxiliary_files
        };
        let _prior = destination.insert(path.clone(), artifact(&bytes));
    }
    validate(&manifest, root)?;
    Ok(manifest)
}

/// Materialize all bytes before either contestant runs and seal the exact lint input inventory.
pub(crate) fn materialize(
    boundary: &Path,
    context: &CaseContext,
    sources: &BTreeMap<ryo::linter::RelativePath, ryo::linter::SourceText>,
) -> Result<ContextManifest, Error> {
    relative(&context.logical_root, PathScope::Root)?;
    let mut manifest = ContextManifest {
        boundary: std::path::absolute(boundary)?,
        logical_root: context.logical_root.clone(),
        auxiliary_files: BTreeMap::new(),
        source_files: BTreeMap::new(),
        missing: context.missing.clone(),
    };
    match std::fs::remove_dir_all(&manifest.boundary) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    std::fs::create_dir_all(manifest.boundary.join(&manifest.logical_root))?;
    for (path, bytes) in &context.auxiliary_files {
        relative(path, PathScope::Member)?;
        let actual = manifest.boundary.join(path);
        std::fs::create_dir_all(
            actual
                .parent()
                .ok_or_else(|| Error::Invalid(constants::MISSING_CONTEXT_PARENT.into()))?,
        )?;
        std::fs::write(actual, &bytes.0)?;
        let _previous = manifest
            .auxiliary_files
            .insert(path.clone(), artifact(bytes));
    }
    for (path, source) in sources {
        let path = resolve(&manifest, Path::new(path.0.as_ref()))?;
        if manifest.auxiliary_files.contains_key(&path) {
            return Err(Error::Invalid(
                constants::AUXILIARY_FILE_ALSO_REQUESTED_FOR_LINT.into(),
            ));
        }
        let actual = manifest.boundary.join(&path);
        std::fs::create_dir_all(
            actual
                .parent()
                .ok_or_else(|| Error::Invalid(constants::MISSING_SOURCE_PARENT.into()))?,
        )?;
        let bytes = ContextBytes(source.0.as_bytes().to_vec());
        std::fs::write(actual, &bytes.0)?;
        let _previous = manifest.source_files.insert(path, artifact(&bytes));
    }
    validate(&manifest, &manifest.boundary.join(&manifest.logical_root))?;
    Ok(manifest)
}

/// Seal observed absences against the same filesystem snapshot as declared files.
/// A missing observation is admissible only after the boundary verifies that path is absent.
pub(crate) fn receipt(
    manifest: &ContextManifest,
    actual_root: &Path,
    dependencies: Vec<ryo::linter::ProjectDependency>,
) -> Result<ProjectContextReceipt, Error> {
    validate(manifest, actual_root)?;
    let mut effective = manifest.clone();
    let mut missing: std::collections::BTreeSet<_> = manifest.missing.iter().cloned().collect();
    let mut observed = BTreeMap::new();
    for dependency in dependencies {
        let path = PathBuf::from(dependency.path.0.as_ref());
        let declared = walked_dependency(manifest, &path)?;
        let state = match dependency.state {
            ryo::linter::ProjectDependencyState::Present(fingerprint) => {
                if !manifest.auxiliary_files.contains_key(&declared)
                    && !manifest.source_files.contains_key(&declared)
                {
                    return Err(Error::Invalid(format!(
                        "undeclared present context dependency: {}",
                        path.display()
                    )));
                }
                let bytes = std::fs::read(manifest.boundary.join(&declared))?;
                if ryo::linter::DependencyFingerprint::from(ryo::linter::DependencyBytes(&bytes))
                    != fingerprint
                {
                    return Err(Error::Invalid(
                        constants::PROJECT_DEPENDENCY_FINGERPRINT_DISAGREES.into(),
                    ));
                }
                Some(fingerprint)
            }
            ryo::linter::ProjectDependencyState::Missing => {
                if manifest.auxiliary_files.contains_key(&declared)
                    || manifest.source_files.contains_key(&declared)
                {
                    return Err(Error::Invalid(format!(
                        "missing context dependency contradicts a sealed file: {}",
                        path.display()
                    )));
                }
                if !missing.contains(&declared) {
                    no_symlinks(&manifest.boundary, &declared)?;
                    match std::fs::symlink_metadata(manifest.boundary.join(&declared)) {
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                        Err(error) => return Err(error.into()),
                        Ok(_) => {
                            return Err(Error::Invalid(format!(
                                "observed missing context path appeared: {}",
                                path.display()
                            )));
                        }
                    }
                    let _new = missing.insert(declared);
                }
                None
            }
        };
        if observed.insert(path.clone(), state).is_some() {
            return Err(Error::Invalid(format!(
                "duplicate context dependency: {}",
                path.display()
            )));
        }
    }
    effective.missing = missing.into_iter().collect();
    Ok(ProjectContextReceipt {
        mode: ProjectContextMode::ListedDisk,
        logical_root: manifest.logical_root.clone(),
        actual_root: actual_root.to_owned(),
        manifest_hash: fingerprint(&effective),
        dependencies: observed,
    })
}

/// Verify the receipt after crossing the worker boundary, including its opaque dependency fingerprints.
pub(crate) fn verify_receipt(
    manifest: &ContextManifest,
    actual_root: &Path,
    observed: &ProjectContextReceipt,
) -> Result<(), Error> {
    let dependencies = observed
        .dependencies
        .iter()
        .map(|(path, state)| ryo::linter::ProjectDependency {
            path: ryo::linter::RelativePath(path.to_string_lossy().into_owned().into_boxed_str()),
            state: state.map_or(
                ryo::linter::ProjectDependencyState::Missing,
                ryo::linter::ProjectDependencyState::Present,
            ),
        })
        .collect();
    if receipt(manifest, actual_root, dependencies)? != *observed {
        return Err(Error::Invalid(
            constants::CONTEXT_RECEIPT_IDENTITY_DISAGREES.into(),
        ));
    }
    Ok(())
}

/// The local filesystem context adapter.
#[derive(Debug)]
pub(crate) struct ContextLocalAdapter;

impl ContextPort for ContextLocalAdapter {
    fn fingerprint(&self, manifest: &ContextManifest) -> ArtifactDigest {
        fingerprint(manifest)
    }
    fn validate(&self, manifest: &ContextManifest, actual_root: &Path) -> Result<(), Error> {
        validate(manifest, actual_root)
    }
    fn seal_existing(
        &self,
        root: &Path,
        sources: &[ryo::linter::RelativePath],
        tracked: CorpusFiles<'_>,
    ) -> Result<ContextManifest, Error> {
        seal_existing(root, sources, tracked.0)
    }
    fn materialize(
        &self,
        boundary: &Path,
        context: &CaseContext,
        sources: &BTreeMap<ryo::linter::RelativePath, ryo::linter::SourceText>,
    ) -> Result<ContextManifest, Error> {
        materialize(boundary, context, sources)
    }
    fn receipt(
        &self,
        manifest: &ContextManifest,
        actual_root: &Path,
        dependencies: Vec<ryo::linter::ProjectDependency>,
    ) -> Result<ProjectContextReceipt, Error> {
        receipt(manifest, actual_root, dependencies)
    }
    fn verify_receipt(
        &self,
        manifest: &ContextManifest,
        actual_root: &Path,
        observed: &ProjectContextReceipt,
    ) -> Result<(), Error> {
        verify_receipt(manifest, actual_root, observed)
    }
    fn receipt_text(&self, receipt: &ProjectContextReceipt) -> Result<String, Error> {
        super::schemas::receipt_text(receipt)
    }
}
