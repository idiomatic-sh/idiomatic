//! The `paths` service: free functions named by verb over the repository's paths, with no IO and no ports.

use super::constants::{CURRENT, NOTHING, PARENT, SEPARATOR};
use super::types::{Containment, PathText, Relative, RepoPath, Segment};

/// The root of the repository: the empty path.
#[must_use]
pub fn root() -> RepoPath {
    RepoPath(Box::default())
}

/// `text` as a path of the repository: separators folded to `/`, `.` segments and empty segments dropped; none when it
/// climbs above the root with `..`.
#[must_use]
pub fn normalize(text: PathText<'_>) -> Option<RepoPath> {
    let mut segments: Vec<&str> = Vec::new();
    for segment in text.0.split(['/', '\\']) {
        match segment {
            NOTHING | CURRENT => {}
            PARENT => {
                let _: &str = segments.pop()?;
            }
            other => segments.push(other),
        }
    }
    Some(RepoPath(segments.join(SEPARATOR).into()))
}

/// `child`, relative, beneath `directory`; none when `child` climbs above the root.
#[must_use]
pub fn join(directory: &RepoPath, child: PathText<'_>) -> Option<RepoPath> {
    if directory.0.is_empty() {
        return normalize(child);
    }
    normalize(PathText(&format!("{}{SEPARATOR}{}", directory.0, child.0)))
}

/// The directory holding `path`; none for the root.
#[must_use]
pub fn parent(path: &RepoPath) -> Option<RepoPath> {
    if path.0.is_empty() {
        return None;
    }
    Some(RepoPath(
        path.0
            .rsplit_once('/')
            .map_or(NOTHING, |(parent, _)| parent)
            .into(),
    ))
}

/// The last segment of `path`: the file or directory name; empty for the root.
#[must_use]
pub fn name(path: &RepoPath) -> Segment {
    Segment(path.0.rsplit('/').next().unwrap_or_default().into())
}

/// Whether `path` is `directory` or lies beneath it.
#[must_use]
pub fn containment(path: &RepoPath, directory: &RepoPath) -> Containment {
    let inside = directory.0.is_empty()
        || path.0 == directory.0
        || path
            .0
            .strip_prefix(&*directory.0)
            .is_some_and(|rest| rest.starts_with('/'));
    if inside {
        Containment::Inside
    } else {
        Containment::Outside
    }
}

/// Every directory holding `path`, the nearest first, the root last.
#[must_use]
pub fn ancestors(path: &RepoPath) -> Vec<RepoPath> {
    let mut found = Vec::new();
    let mut current = parent(path);
    while let Some(directory) = current {
        current = parent(&directory);
        found.push(directory);
    }
    found
}

/// `path` relative to `base`: none when it lies elsewhere; the empty text for the base itself.
#[must_use]
pub fn relative<'a>(path: &'a RepoPath, base: &RepoPath) -> Option<Relative<'a>> {
    if base.0.is_empty() {
        return Some(Relative(&path.0));
    }
    if path.0 == base.0 {
        return Some(Relative(NOTHING));
    }
    path.0
        .strip_prefix(&*base.0)?
        .strip_prefix(SEPARATOR)
        .map(Relative)
}

/// The sorted `files` at or beneath `directory`: every file for the root.
#[must_use]
pub fn beneath<'f>(files: &'f [RepoPath], directory: &RepoPath) -> &'f [RepoPath] {
    if directory.0.is_empty() {
        return files;
    }
    let prefix = format!("{}{SEPARATOR}", directory.0);
    let start = files.partition_point(|file| *file.0 < *prefix);
    let tail = files.get(start..).unwrap_or_default();
    let end = tail.partition_point(|file| file.0.starts_with(&prefix));
    tail.get(..end).unwrap_or_default()
}

/// Every directory holding one of `files`, and each of their ancestors, the root included, sorted.
#[must_use]
pub fn directories(files: &[RepoPath]) -> Vec<RepoPath> {
    let mut found = std::collections::BTreeSet::new();
    for file in files {
        for ancestor in ancestors(file) {
            if !found.insert(ancestor) {
                break;
            }
        }
    }
    found.into_iter().collect()
}
