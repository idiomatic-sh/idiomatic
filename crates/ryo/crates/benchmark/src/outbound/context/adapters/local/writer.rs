//! The adapter's traversal of the snapshot it seals: lexical path resolution and symlink walks.
use super::constants;
use benchmark_contracts::domain::context::types::{ContextManifest, LinkTarget};
use benchmark_contracts::domain::reports::errors::ReportsError as Error;
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

/// The exact target text of the link at `actual`, as the filesystem holds it; never normalized.
pub(super) fn link_text(actual: &Path) -> Result<LinkTarget, Error> {
    std::fs::read_link(actual)?
        .into_os_string()
        .into_string()
        .map(LinkTarget)
        .map_err(|_text| Error::Invalid(constants::NON_UTF8_LINK_TARGET.into()))
}

/// The sealed path a link's target text names, read from the raw text and never from normalizing path components:
/// the link's directory, one parent popped per leading `..` segment, then the names that follow.
/// The grammar is slash-only: refuses empty text, a leading, repeated or trailing `/`, any `.` segment, `..` after a
/// name, an escape, and a segment holding a backslash or any other separator the platform's paths know.
pub(super) fn link_destination(link: &Path, target: &LinkTarget) -> Result<PathBuf, Error> {
    let refuse = || Error::Invalid(constants::CONTEXT_LINK_TARGET_NOT_CANONICAL.into());
    let mut destination = link.parent().map(Path::to_path_buf).unwrap_or_default();
    let mut named = false;
    for raw in target.0.split('/') {
        if raw
            .chars()
            .any(|character| character == '\\' || std::path::is_separator(character))
        {
            return Err(refuse());
        }
        let segment = std::ffi::OsStr::new(raw);
        if segment.is_empty() || segment == Component::CurDir.as_os_str() {
            return Err(refuse());
        }
        if segment == Component::ParentDir.as_os_str() {
            if named || !destination.pop() {
                return Err(refuse());
            }
        } else {
            named = true;
            destination.push(segment);
        }
    }
    relative(&destination, PathScope::Member)?;
    Ok(destination)
}

/// Reject symlinks on the way to a path, the path itself excepted: the walk a sealed link's own directory must pass.
pub(super) fn no_symlinked_parent(boundary: &Path, relative: &Path) -> Result<(), Error> {
    relative
        .parent()
        .map_or(Ok(()), |parent| no_symlinks(boundary, parent))
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

/// Refuse a path beneath a file: a component of `relative` that exists and is not a directory, where the platform does not
/// itself answer a path under a file with an error other than not-found.
pub(super) fn no_file_ancestors(boundary: &Path, relative: &Path) -> Result<(), Error> {
    let mut path = boundary.to_owned();
    let mut parts = relative.components().peekable();
    while let Some(part) = parts.next() {
        path.push(part.as_os_str());
        if parts.peek().is_none() {
            break;
        }
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if !metadata.is_dir() => {
                return Err(Error::Invalid(format!(
                    "context path beneath a file: {}",
                    path.display()
                )));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}
