//! The adapter's traversal of the snapshot it seals: lexical path resolution and symlink walks.
use super::constants;
use ryo_benchmark_contracts::domain::context::types::ContextManifest;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::path::{Component, Path, PathBuf};

/// Whether a validated path names the logical root or a member beneath it.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PathScope {
    Root,
    Member,
}

/// Validate a path wholly inside a snapshot; `.` is allowed only for the logical root.
pub(super) fn relative(path: &Path, scope: PathScope) -> Result<(), Error> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_) | Component::CurDir))
        || scope == PathScope::Member && path.components().all(|part| part == Component::CurDir)
    {
        return Err(Error::Invalid(format!(
            "unsafe context path: {}",
            path.display()
        )));
    }
    Ok(())
}

/// Resolve a root-relative dependency lexically, without allowing it to escape its snapshot.
pub(super) fn resolve(manifest: &ContextManifest, path: &Path) -> Result<PathBuf, Error> {
    if path.is_absolute() {
        return Err(Error::Invalid(
            constants::ABSOLUTE_CONTEXT_DEPENDENCY.into(),
        ));
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
    relative(&resolved, PathScope::Member)?;
    Ok(resolved)
}

/// Preserve the filesystem meaning of the original dependency path before lexical resolution.
/// A cancelled component must be an existing directory, and no walked component may be a symlink.
pub(super) fn walked_dependency(manifest: &ContextManifest, path: &Path) -> Result<PathBuf, Error> {
    let resolved = resolve(manifest, path)?;
    let mut walked = manifest.boundary.join(&manifest.logical_root);
    for part in path.components() {
        match part {
            Component::Normal(name) => {
                walked.push(name);
                match std::fs::symlink_metadata(&walked) {
                    Ok(metadata) if metadata.file_type().is_symlink() => {
                        return Err(Error::Invalid(format!(
                            "symlink in context dependency: {}",
                            path.display()
                        )));
                    }
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.into()),
                }
            }
            Component::ParentDir => {
                let metadata = std::fs::symlink_metadata(&walked)?;
                if !metadata.is_dir() || metadata.file_type().is_symlink() {
                    return Err(Error::Invalid(format!(
                        "non-directory in context dependency: {}",
                        path.display()
                    )));
                }
                let _parent = walked.pop();
            }
            Component::CurDir => {}
            Component::Prefix(_) | Component::RootDir => {
                return Err(Error::Invalid(format!(
                    "unsafe context dependency: {}",
                    path.display()
                )));
            }
        }
    }
    Ok(resolved)
}

/// Reject symlinks within the snapshot, including missing paths with a symlinked parent.
pub(super) fn no_symlinks(boundary: &Path, relative: &Path) -> Result<(), Error> {
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
