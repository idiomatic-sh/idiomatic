//! Materialize and validate explicitly sealed project context at the filesystem boundary.
use crate::domain::diagnostics::{errors::DiagnosticsError as Error, types::ProjectContextReceipt};
use crate::domain::suite::types::{
    ArtifactDigest, CaseContext, ContextManifest, ProjectContextMode,
};
use sha2::{Digest as _, Sha256};
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

/// Raw artifact identity, independent of the engine's opaque dependency fingerprint.
pub(crate) fn artifact(bytes: &[u8]) -> ArtifactDigest {
    ArtifactDigest(format!("{:x}", Sha256::digest(bytes)))
}

/// Validate a path wholly inside a snapshot; `.` is allowed only for the logical root.
fn relative(path: &Path, root: bool) -> Result<(), Error> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_) | Component::CurDir))
        || !root && path.components().all(|part| part == Component::CurDir)
    {
        return Err(Error::Invalid(format!(
            "unsafe context path: {}",
            path.display()
        )));
    }
    Ok(())
}

/// Resolve a root-relative dependency lexically, without allowing it to escape its snapshot.
fn resolve(manifest: &ContextManifest, path: &Path) -> Result<PathBuf, Error> {
    if path.is_absolute() {
        return Err(Error::Invalid("absolute context dependency".into()));
    }
    let mut resolved = PathBuf::new();
    for part in manifest.logical_root.components() {
        if let Component::Normal(name) = part {
            resolved.push(name);
        }
    }
    for part in path.components() {
        match part {
            Component::Normal(name) => resolved.push(name),
            Component::CurDir => {}
            Component::ParentDir if resolved.pop() => {}
            Component::Prefix(_) | Component::RootDir | Component::ParentDir => {
                return Err(Error::Invalid(format!(
                    "context dependency escapes boundary: {}",
                    path.display()
                )));
            }
        }
    }
    relative(&resolved, false)?;
    Ok(resolved)
}

/// Reject symlinks within the snapshot, including missing paths with a symlinked parent.
fn no_symlinks(boundary: &Path, relative: &Path) -> Result<(), Error> {
    let mut path = boundary.to_owned();
    for part in std::iter::once(None).chain(relative.components().map(Some)) {
        if let Some(part) = part {
            path.push(part.as_os_str());
        }
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(Error::Invalid(format!(
                    "symlink in context: {}",
                    path.display()
                )));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

/// Content seal includes declared absences and stable logical location, never temporary physical directories.
pub(crate) fn fingerprint(manifest: &ContextManifest) -> ArtifactDigest {
    let mut hash = Sha256::new();
    frame(&mut hash, b"listed-disk-context-v1");
    frame(
        &mut hash,
        manifest.logical_root.as_os_str().as_encoded_bytes(),
    );
    for (kind, files) in [
        (b"source".as_slice(), &manifest.source_files),
        (b"auxiliary".as_slice(), &manifest.auxiliary_files),
    ] {
        frame(&mut hash, kind);
        for (path, digest) in files {
            frame(&mut hash, path.as_os_str().as_encoded_bytes());
            frame(&mut hash, digest.0.as_bytes());
        }
    }
    let mut missing = manifest.missing.clone();
    missing.sort();
    for path in missing {
        frame(&mut hash, b"missing");
        frame(&mut hash, path.as_os_str().as_encoded_bytes());
    }
    ArtifactDigest(format!("{:x}", hash.finalize()))
}

/// Validate found and missing dependencies without treating unreadable files as missing.
pub(crate) fn validate(manifest: &ContextManifest, actual_root: &Path) -> Result<(), Error> {
    relative(&manifest.logical_root, true)?;
    if !manifest.boundary.is_absolute()
        || actual_root != manifest.boundary.join(&manifest.logical_root)
    {
        return Err(Error::Invalid(
            "context logical and physical roots disagree".into(),
        ));
    }
    let mut missing = manifest.missing.clone();
    missing.sort();
    if missing
        .windows(2)
        .any(|pair| matches!(pair, [first, second] if first == second))
    {
        return Err(Error::Invalid("duplicate missing context path".into()));
    }
    for (path, expected) in manifest
        .source_files
        .iter()
        .chain(&manifest.auxiliary_files)
    {
        relative(path, false)?;
        no_symlinks(&manifest.boundary, path)?;
        if manifest.source_files.contains_key(path) && manifest.auxiliary_files.contains_key(path)
            || missing.contains(path)
        {
            return Err(Error::Invalid("context path has conflicting roles".into()));
        }
        let bytes = std::fs::read(manifest.boundary.join(path))?;
        if artifact(&bytes) != *expected {
            return Err(Error::Invalid(format!(
                "context bytes changed: {}",
                path.display()
            )));
        }
    }
    for path in &missing {
        relative(path, false)?;
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
            "sealed corpus requires unique actual tracked paths covering every requested source"
                .into(),
        ));
    }
    let mut manifest = ContextManifest {
        boundary,
        logical_root: PathBuf::from("."),
        source_files: BTreeMap::new(),
        auxiliary_files: BTreeMap::new(),
        missing: Vec::new(),
    };
    for path in tracked {
        relative(path, false)?;
        no_symlinks(&manifest.boundary, path)?;
        let actual = manifest.boundary.join(path);
        let bytes = std::fs::read(&actual).map_err(|error| {
            Error::Io(std::io::Error::new(
                error.kind(),
                format!("seal corpus {}: {error}", actual.display()),
            ))
        })?;
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
    relative(&context.logical_root, true)?;
    let mut manifest = ContextManifest {
        boundary: std::path::absolute(boundary)?,
        logical_root: context.logical_root.clone(),
        auxiliary_files: BTreeMap::new(),
        source_files: BTreeMap::new(),
        missing: context.missing.clone(),
    };
    if manifest.boundary.exists() {
        std::fs::remove_dir_all(&manifest.boundary)?;
    }
    std::fs::create_dir_all(manifest.boundary.join(&manifest.logical_root))?;
    for (path, bytes) in &context.auxiliary_files {
        relative(path, false)?;
        let actual = manifest.boundary.join(path);
        std::fs::create_dir_all(
            actual
                .parent()
                .ok_or_else(|| Error::Invalid("missing context parent".into()))?,
        )?;
        std::fs::write(actual, bytes)?;
        let _previous = manifest
            .auxiliary_files
            .insert(path.clone(), artifact(bytes));
    }
    for (path, source) in sources {
        let path = resolve(&manifest, Path::new(path.0.as_ref()))?;
        if manifest.auxiliary_files.contains_key(&path) {
            return Err(Error::Invalid(
                "auxiliary file also requested for lint".into(),
            ));
        }
        let actual = manifest.boundary.join(&path);
        std::fs::create_dir_all(
            actual
                .parent()
                .ok_or_else(|| Error::Invalid("missing source parent".into()))?,
        )?;
        std::fs::write(actual, source.0.as_bytes())?;
        let _previous = manifest
            .source_files
            .insert(path, artifact(source.0.as_bytes()));
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
        let declared = resolve(manifest, &path)?;
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
                if ryo::linter::dependency_fingerprint(&bytes) != fingerprint {
                    return Err(Error::Invalid(
                        "project dependency fingerprint disagrees".into(),
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
        return Err(Error::Invalid("context receipt identity disagrees".into()));
    }
    Ok(())
}

/// Hash one length-framed field without delimiter ambiguity.
fn frame(hash: &mut Sha256, bytes: &[u8]) {
    hash.update(u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_le_bytes());
    hash.update(bytes);
}
