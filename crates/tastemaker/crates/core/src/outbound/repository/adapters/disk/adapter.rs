//! The repository port, implemented over the disk: `ignore` walks it, `.gitignore` applying.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::constants::{GIT, NOT_UTF8, TASTEMAKER, WORKSPACE_FILE};
use super::types::{Kept, RepositoryDiskProject, RepositoryDiskRepository};
use crate::domain::files::types::{
    Declarer, DirectoryName, FileText, MachinePath, SkippedDirectories, SkippedDirectory, Stamp,
};
use crate::domain::paths::service::normalize;
use crate::domain::paths::types::{PathText, RepoPath};
use crate::outbound::repository::errors::RepositoryError;
use crate::outbound::repository::interface::{FoundRepository, Repository, RepositoryPort};
use crate::outbound::repository::types::{Marker, Presence, RepoPaths, Walk};

/// The disk repository adapter, and the directory names the linked languages declare skipped, beside the ones Tastemaker always
/// skips.
#[derive(Debug)]
pub(crate) struct RepositoryDiskAdapter {
    /// The directories a walk never goes into, at any depth, from the linked languages.
    pub skipped: Vec<SkippedDirectory>,
    /// The store's directory, never walked: it is no file of the project.
    pub store: DirectoryName,
}

impl RepositoryDiskAdapter {
    /// The directory `root`, bound, walked as this adapter walks.
    fn bound(&self, root: MachinePath) -> RepositoryDiskRepository {
        RepositoryDiskRepository {
            root,
            skipped: self.skipped.clone(),
            store: self.store.clone(),
            workspace: RepoPath::default(),
            start: RepoPath::default(),
        }
    }
}

/// Where `path` is on disk, within `repository`.
fn location(repository: &MachinePath, path: &RepoPath) -> PathBuf {
    let root = Path::new(&*repository.0);
    if path.0.is_empty() {
        root.to_path_buf()
    } else {
        root.join(&*path.0)
    }
}

/// Whether `path` is on disk: only a path that is not there (`NotFound`) is absent; one whose metadata cannot be read
/// (a link to itself, a directory that cannot be searched) is there, so reading it says why it cannot be read, never that
/// it is missing.
fn presence(path: &Path) -> Presence {
    match std::fs::metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Presence::Absent,
        Ok(_) | Err(_) => Presence::Present,
    }
}

/// The nearest directory at or above `start` that holds `name`.
fn nearest(start: &Path, name: Marker<'_>) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|directory| presence(&directory.join(name.0)) == Presence::Present)
        .map(Path::to_path_buf)
}

impl RepositoryPort for RepositoryDiskAdapter {
    fn repository(&self, start: &MachinePath) -> Result<MachinePath, RepositoryError> {
        let resolved = std::fs::canonicalize(&*start.0).map_err(|error| RepositoryError::Read {
            path: start.0.to_string(),
            reason: error.to_string(),
        })?;
        nearest(&resolved, Marker(GIT))
            .map(|repository| MachinePath(repository.display().to_string().into()))
            .ok_or_else(|| RepositoryError::NoRepository {
                workspace: resolved.display().to_string(),
            })
    }

    fn find(&self, start: &MachinePath) -> Result<Arc<dyn FoundRepository>, RepositoryError> {
        let start = std::fs::canonicalize(&*start.0).map_err(|error| RepositoryError::Read {
            path: start.0.to_string(),
            reason: error.to_string(),
        })?;
        let no_workspace = || RepositoryError::NoWorkspace {
            start: start.display().to_string(),
        };
        let Some(repository) = nearest(&start, Marker(GIT)) else {
            let workspace = nearest(&start, Marker(WORKSPACE_FILE)).ok_or_else(no_workspace)?;
            return Err(RepositoryError::NoRepository {
                workspace: workspace.display().to_string(),
            });
        };
        let workspace = start
            .ancestors()
            .take_while(|directory| directory.starts_with(&repository))
            .find(|directory| presence(&directory.join(WORKSPACE_FILE)) == Presence::Present)
            .map(Path::to_path_buf)
            .ok_or_else(no_workspace)?;
        // Both lie at or beneath the repository, found among their ancestors; a name that is not UTF-8 is refused.
        let within = |path: &Path| {
            let rest = path
                .strip_prefix(&repository)
                .map_err(|error| RepositoryError::Read {
                    path: path.display().to_string(),
                    reason: error.to_string(),
                })?;
            let text = rest.to_str().ok_or_else(|| RepositoryError::Read {
                path: path.display().to_string(),
                reason: NOT_UTF8.to_owned(),
            })?;
            Ok::<_, RepositoryError>(normalize(PathText(text)).unwrap_or_default())
        };
        let relative = within(&workspace)?;
        let start = within(&start)?;
        Ok(Arc::new(RepositoryDiskRepository {
            workspace: relative,
            start,
            ..self.bound(MachinePath(repository.display().to_string().into()))
        }))
    }

    fn open(&self, path: &MachinePath) -> Arc<dyn Repository> {
        Arc::new(self.bound(path.clone()))
    }

    fn skipped(&self) -> SkippedDirectories {
        let mut skipped: Vec<SkippedDirectory> = [GIT, &self.store.0]
            .into_iter()
            .map(|name| SkippedDirectory {
                name: DirectoryName(name.into()),
                by: vec![Declarer(TASTEMAKER.into())],
            })
            .collect();
        skipped.extend(self.skipped.iter().cloned());
        skipped.sort();
        SkippedDirectories(skipped)
    }

    fn project(&self, root: &MachinePath) -> Result<Box<dyn Repository>, RepositoryError> {
        let path = PathBuf::from(&*root.0);
        let refused = |error: std::io::Error| RepositoryError::Write {
            path: root.0.to_string(),
            reason: error.to_string(),
        };
        if let Some(above) = path.parent() {
            std::fs::create_dir_all(above).map_err(refused)?;
        }
        // Made new, never into a directory that is there: it is removed when the handle is dropped.
        #[expect(
            clippy::create_dir,
            reason = "the directory is made new: one that is there is refused, which create_dir_all would accept"
        )]
        let made = std::fs::create_dir(&path);
        made.map_err(refused)?;
        Ok(Box::new(RepositoryDiskProject {
            repository: self.bound(root.clone()),
        }))
    }
}

impl FoundRepository for RepositoryDiskRepository {
    fn workspace(&self) -> &RepoPath {
        &self.workspace
    }

    fn start(&self) -> &RepoPath {
        &self.start
    }
}

impl Repository for RepositoryDiskRepository {
    fn root(&self) -> &MachinePath {
        &self.root
    }

    fn walk(&self, under: &RepoPath) -> Result<Walk, RepositoryError> {
        let root = Path::new(&*self.root.0);
        let start = location(&self.root, under);
        if presence(&start) == Presence::Absent {
            return Ok(Walk::default());
        }
        let (skipped, store) = (self.skipped.clone(), self.store.clone());
        let walker = ignore::WalkBuilder::new(&start)
            .hidden(false)
            .parents(true)
            .require_git(true)
            .git_global(false)
            .filter_entry(move |entry| {
                let name = entry.file_name();
                // Git's own `.git` is never the project's: a directory in a checkout, a file in a worktree.
                name != GIT
                    && !(entry.file_type().is_some_and(|kind| kind.is_dir())
                        && (name == &*store.0 || skipped.iter().any(|skip| name == &*skip.name.0)))
            })
            .build_parallel();
        // Each walker thread keeps its own files and its first error, handed over when it ends.
        let found = std::sync::Mutex::new(Walk::default());
        let failed = std::sync::Mutex::new(None);
        let shown = start.display().to_string();
        walker.run(|| {
            let shown = shown.clone();
            let mut own = Kept {
                walk: Walk::default(),
                failure: None,
                into: &found,
                failed: &failed,
            };
            Box::new(move |entry| {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(error) => {
                        own.failure = Some(RepositoryError::Read {
                            path: shown.clone(),
                            reason: error.to_string(),
                        });
                        return ignore::WalkState::Quit;
                    }
                };
                // Every entry lies beneath the root the walk started under.
                let relative = match entry.path().strip_prefix(root) {
                    Ok(relative) => relative,
                    Err(error) => {
                        own.failure = Some(RepositoryError::Read {
                            path: entry.path().display().to_string(),
                            reason: error.to_string(),
                        });
                        return ignore::WalkState::Quit;
                    }
                };
                // A name that is not UTF-8 is a file no rule could read or report: it fails the walk, never drops out of it.
                if relative.to_str().is_none() {
                    own.failure = Some(RepositoryError::Read {
                        path: entry.path().display().to_string(),
                        reason: NOT_UTF8.to_owned(),
                    });
                    return ignore::WalkState::Quit;
                }
                let path = relative.to_str().and_then(|path| normalize(PathText(path)));
                if let (Some(kind), Some(path)) = (entry.file_type(), path) {
                    #[expect(
                        clippy::filetype_is_file,
                        reason = "a walk lists regular files only: a fifo, socket or device is no file to read, and `!is_dir` would list what a read blocks on"
                    )]
                    let regular = kind.is_file();
                    // A symbolic link is listed apart, a path of the tree that is never followed.
                    match (regular, kind.is_dir(), kind.is_symlink()) {
                        (true, _, _) => own.walk.files.push(path),
                        (false, true, _) => own.walk.directories.push(path),
                        (false, false, true) => own.walk.links.push(path),
                        (false, false, false) => {}
                    }
                }
                ignore::WalkState::Continue
            })
        });
        let Walk {
            mut files,
            mut directories,
            mut links,
        } = found
            .into_inner()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(failure) = failed
            .into_inner()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
        {
            return Err(failure);
        }
        files.sort();
        directories.sort();
        links.sort();
        Ok(Walk {
            files,
            directories,
            links,
        })
    }

    fn list(&self, under: &RepoPath) -> Result<RepoPaths, RepositoryError> {
        self.walk(under).map(|walk| RepoPaths(walk.files))
    }

    fn read(&self, path: &RepoPath) -> Result<FileText, RepositoryError> {
        std::fs::read_to_string(location(&self.root, path))
            .map(|text| FileText(text.into()))
            .map_err(|error| RepositoryError::Read {
                path: path.0.to_string(),
                reason: error.to_string(),
            })
    }

    fn exists(&self, path: &RepoPath) -> Presence {
        presence(&location(&self.root, path))
    }

    fn write(&self, path: &RepoPath, text: &FileText) -> Result<(), RepositoryError> {
        let target = location(&self.root, path);
        let failure = |error: std::io::Error| RepositoryError::Write {
            path: path.0.to_string(),
            reason: error.to_string(),
        };
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(failure)?;
        }
        std::fs::write(&target, &*text.0).map_err(failure)
    }

    fn remove(&self, path: &RepoPath) -> Result<(), RepositoryError> {
        let target = location(&self.root, path);
        let removed = match std::fs::symlink_metadata(&target) {
            Ok(meta) if meta.is_dir() => std::fs::remove_dir(&target),
            Ok(_) => std::fs::remove_file(&target),
            Err(error) => Err(error),
        };
        removed.map_err(|error| RepositoryError::Write {
            path: path.0.to_string(),
            reason: error.to_string(),
        })
    }

    fn stamps(&self, paths: &[RepoPath]) -> crate::domain::files::types::Stamps {
        let found: Vec<Stamp> = {
            paths
                .iter()
                .map(|path| {
                    let stamped = std::fs::metadata(location(&self.root, path)).and_then(|meta| {
                        let modified = meta
                            .modified()?
                            .duration_since(std::time::UNIX_EPOCH)
                            .map_err(std::io::Error::other)?
                            .as_nanos();
                        Ok(Stamp {
                            size: crate::domain::files::types::FileSize(meta.len()),
                            modified: crate::domain::files::types::ModifiedTime(
                                u64::try_from(modified).unwrap_or(u64::MAX),
                            ),
                        })
                    });
                    // A file whose size or time cannot be read has no stamp, and is read again.
                    stamped.unwrap_or_default()
                })
                .collect()
        };
        crate::domain::files::types::Stamps(found)
    }
}

impl Repository for RepositoryDiskProject {
    fn root(&self) -> &MachinePath {
        self.repository.root()
    }

    fn list(&self, under: &RepoPath) -> Result<RepoPaths, RepositoryError> {
        self.repository.list(under)
    }

    fn walk(&self, under: &RepoPath) -> Result<Walk, RepositoryError> {
        self.repository.walk(under)
    }

    fn read(&self, path: &RepoPath) -> Result<FileText, RepositoryError> {
        self.repository.read(path)
    }

    fn exists(&self, path: &RepoPath) -> Presence {
        self.repository.exists(path)
    }

    fn write(&self, path: &RepoPath, text: &FileText) -> Result<(), RepositoryError> {
        self.repository.write(path, text)
    }

    fn remove(&self, path: &RepoPath) -> Result<(), RepositoryError> {
        self.repository.remove(path)
    }

    fn stamps(&self, paths: &[RepoPath]) -> crate::domain::files::types::Stamps {
        self.repository.stamps(paths)
    }
}
