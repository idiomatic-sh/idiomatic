//! The `paths` service: free functions named by verb that place and cut the paths of a repository, with no IO and no ports.

use super::constants::{CUTS, DIRECTORY_OWNER, HERE, MANIFEST, SEPARATOR, UP};

use super::types::{Asked, Linking, Lying, Ownership, Relative, Settlement, Supply};
use crate::domain::paths::types::EntryName;
use crate::domain::paths::types::Height;
use crate::domain::paths::types::RepoPath;
use crate::domain::paths::types::Section;
use crate::domain::paths::types::{Entry, EntryKind};

/// What a section holds, none when it is absent.
pub(crate) fn present<T>(section: Section<T>) -> Option<T> {
    match section {
        Section::Present(value) => Some(value),
        Section::Absent => None,
    }
}

/// The directory a path is in; the root for a path of the root.
#[must_use]
pub(crate) fn directory_of(path: &RepoPath) -> RepoPath {
    RepoPath(
        path.rsplit_once(SEPARATOR)
            .map(|(directory, _)| directory)
            .unwrap_or_default()
            .into(),
    )
}

/// The last part of a path.
#[must_use]
pub(crate) fn name_of(path: &RepoPath) -> EntryName {
    EntryName(path.rsplit(SEPARATOR).next().unwrap_or_default().into())
}

/// `relative` placed under `directory`, with `.` and `..` resolved; nothing when it climbs out of the repository.
#[must_use]
pub(crate) fn join(directory: &RepoPath, relative: &Relative) -> Option<RepoPath> {
    let mut parts: Vec<&str> = directory
        .split(SEPARATOR)
        .filter(|part| !part.is_empty())
        .collect();
    for part in relative.0.split(CUTS) {
        if part.is_empty() || part == HERE {
            continue;
        }
        if part == UP {
            parts.truncate(parts.len().checked_sub(1)?);
        } else {
            parts.push(part);
        }
    }
    Some(RepoPath(parts.join(SEPARATOR).into()))
}

/// `relative` placed under `directory` as it is written: Cargo keeps a declared path of a target as it was declared, `..` and all.
#[must_use]
pub(crate) fn written(directory: &RepoPath, relative: &Relative) -> RepoPath {
    let relative: &str = &relative.0;
    let relative = relative
        .strip_prefix(HERE)
        .and_then(|rest| rest.strip_prefix(SEPARATOR))
        .unwrap_or(relative);
    if directory.is_empty() {
        RepoPath(relative.into())
    } else {
        RepoPath([&**directory, SEPARATOR, relative].concat().into())
    }
}

/// The manifest in a directory.
#[must_use]
pub(crate) fn manifest_in(directory: &RepoPath) -> RepoPath {
    written(directory, &Relative(MANIFEST.into()))
}

/// Whether `path` is `directory` or lies below it.
#[must_use]
pub(crate) fn within(path: &RepoPath, directory: &RepoPath) -> Lying {
    if directory.is_empty()
        || path == directory
        || path
            .strip_prefix(&**directory)
            .is_some_and(|rest| rest.starts_with(SEPARATOR))
    {
        Lying::Within
    } else {
        Lying::Apart
    }
}

/// `file`'s name without its extension.
#[must_use]
pub(crate) fn stem(file: &RepoPath) -> EntryName {
    let name = file.rsplit(SEPARATOR).next().unwrap_or(&**file);
    EntryName(name.rsplit_once('.').map_or(name, |(stem, _)| stem).into())
}

/// Whether `file` owns its directory as a module: `mod.rs`.
#[must_use]
pub(crate) fn owns_directory(file: &RepoPath) -> Ownership {
    if file.rsplit(SEPARATOR).next() == Some(DIRECTORY_OWNER) {
        Ownership::Owns
    } else {
        Ownership::Shares
    }
}

/// `relative` under `base`, with `.` and `..` resolved.
#[must_use]
pub(crate) fn lexical_join(base: &RepoPath, relative: &Relative) -> RepoPath {
    let mut parts: Vec<&str> = base
        .split(SEPARATOR)
        .filter(|part| !part.is_empty())
        .collect();
    for part in relative.split(SEPARATOR) {
        match part {
            part if part.is_empty() || part == HERE => {}
            part if part == UP => parts.truncate(parts.len().saturating_sub(1)),
            other => parts.push(other),
        }
    }
    RepoPath(parts.join(SEPARATOR).into())
}

/// The path from the root of the file `name` in the directory `height` above `directory`: one `..` for each directory above the
/// root it lies in.
#[must_use]
pub(crate) fn found_above(directory: &RepoPath, height: Height, name: &EntryName) -> RepoPath {
    let mut parts: Vec<&str> = directory
        .split(SEPARATOR)
        .filter(|part| !part.is_empty())
        .collect();
    let climbed = usize::try_from(height.0).unwrap_or(usize::MAX);
    match parts.len().checked_sub(climbed) {
        Some(kept) => parts.truncate(kept),
        None => parts = vec![UP; climbed.saturating_sub(parts.len())],
    }
    parts.push(&name.0);
    RepoPath(parts.join(SEPARATOR).into())
}

/// What `asked` was told of `key`; `pending` when nothing yet, and `key` is wanted.
pub(crate) fn told<K, V>(asked: &mut Asked<K, V>, key: &K, pending: impl FnOnce() -> V) -> V
where
    K: Ord + Clone,
    V: Clone,
{
    if let Some(seen) = asked.seen.get(key) {
        return seen.clone();
    }
    asked.wanted.extend([key.clone()]);
    pending()
}

/// Whether `asked` wants nothing told.
#[must_use]
pub(crate) fn settlement_of<K, V>(asked: &Asked<K, V>) -> Settlement
where
    K: Ord,
{
    if asked.wanted.is_empty() {
        Settlement::Settled
    } else {
        Settlement::Wanting
    }
}

/// What is in `directory`, from the paths of the files a request lists and of the manifests it supplies: each name below it once, a
/// directory when a path goes on below the name.
#[must_use]
pub(crate) fn entries_among(
    files: &[ryo::languages::FilePath],
    supplied: &[ryo::languages::ManifestFile],
    directory: &RepoPath,
) -> Vec<Entry> {
    let mut found: Vec<Entry> = Vec::new();
    let paths = files
        .iter()
        .map(|file| &*file.0)
        .chain(supplied.iter().map(|manifest| &*manifest.path.0));
    for path in paths {
        let rest = if directory.0.is_empty() {
            Some(path)
        } else {
            path.strip_prefix(&*directory.0)
                .and_then(|rest| rest.strip_prefix(SEPARATOR))
        };
        let Some(rest) = rest else {
            continue;
        };
        let (name, kind) = match rest.split_once(SEPARATOR) {
            Some((name, _)) => (name, EntryKind::Directory),
            None => (rest, EntryKind::File),
        };
        if !name.is_empty() && !found.iter().any(|each| &*each.name == name) {
            found.push(Entry {
                name: EntryName(name.into()),
                kind,
            });
        }
    }
    found
}

/// Tells `asked` what it wanted: `answer` gives the value of each key the pass met unseen, and the pass is run again.
pub(crate) fn fulfil<K: Ord, V>(asked: &mut Asked<K, V>, mut answer: impl FnMut(&K) -> V) {
    for key in std::mem::take(&mut asked.wanted) {
        let value = answer(&key);
        drop(asked.seen.insert(key, value));
    }
}

/// Whether the manifests a run supplies include a manifest of the language.
#[must_use]
pub(crate) fn supply_of(manifests: &[ryo::languages::ManifestFile]) -> Supply {
    if manifests
        .iter()
        .any(|manifest| *name_of(&RepoPath(manifest.path.0.clone())).0 == *MANIFEST)
    {
        Supply::Manifest
    } else {
        Supply::NoManifest
    }
}

/// The files a run consulted, in order and each once.
#[must_use]
pub(crate) fn consulted_files(mut found: Vec<RepoPath>) -> Vec<ryo::languages::FilePath> {
    found.sort_unstable();
    found.dedup();
    found
        .into_iter()
        .map(|path| ryo::languages::FilePath(path.0))
        .collect()
}

/// Whether any of the relations asked for links the source files to one another.
#[must_use]
pub(crate) fn linking_of(wanted: &[ryo::languages::Relation]) -> Linking {
    if wanted.iter().any(|each| {
        matches!(
            each,
            ryo::languages::Relation::SourceDependency | ryo::languages::Relation::AnalyzedSource
        )
    }) {
        Linking::Asked
    } else {
        Linking::NotAsked
    }
}
