//! The fake of the `repository` port: a repository held in memory, its files by path.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use super::project::ProjectFake;
use crate::domain::files::types::{
    Declarer, DirectoryName, FileText, MachinePath, SkippedDirectories, SkippedDirectory, Stamp,
};
use crate::domain::paths::service::containment;
use crate::domain::paths::types::{Containment, RepoPath};
use crate::outbound::repository::errors::RepositoryError;
use crate::outbound::repository::interface::{FoundRepository, Repository, RepositoryPort};
use crate::outbound::repository::types::{Presence, RepoPaths, Walk};

/// The directory every repository skips, as the disk adapter does: Tastemaker's own, declared by Tastemaker.
const VERSION_CONTROL: &str = ".git";

/// The root of the fake's own repository, which `find` and `repository` answer outside a scratch directory.
const FAKE_ROOT: &str = "/fake";

/// What the core's tests stand in for `repository` with: the files by path, and the workspace's directory. It is also the
/// repository it hands out, bound to `root`: a scratch directory's files when `root` is in one, else its own.
#[derive(Clone, Debug, Default)]
pub struct RepositoryFake {
    /// The root it is bound to as a repository: `/fake` for its own files.
    pub root: MachinePath,
    /// The files, shared by every clone: a test keeps one to change the files a use case reads.
    pub files: Arc<Mutex<BTreeMap<RepoPath, FileText>>>,
    /// The workspace `find` answers.
    pub workspace: FakeWorkspace,
    /// Where the nearest `tastemaker.toml` is, once `find` bound the fake, and where the search began; empty otherwise.
    // ignore: idiomatic.arch/pub-item-unreferenced-elsewhere - read only by this fake's own `FoundRepository` methods; the fields are all `pub` for clippy's partial_pub_fields
    pub found_in: RepoPath,
    /// The directories the fake says its walks skip beside `.git` (it lists the files it holds as given).
    pub skipped: Vec<SkippedDirectory>,
    /// Paths whose writes are refused, to exercise partial write failures.
    pub write_failures: BTreeSet<RepoPath>,
    /// Paths that are there but whose reads fail, as a file the system cannot read.
    pub read_failures: BTreeSet<RepoPath>,
    /// Every path read, in order, shared by every clone: a test reads it to see what a run read.
    pub reads: Arc<Mutex<Vec<RepoPath>>>,
    /// How many scratch directories it has made, so each is a new one.
    pub scratched: Arc<std::sync::atomic::AtomicUsize>,
    /// The files of each scratch directory, apart from the repository's and from one another, as on a disk.
    pub scratches: Scratches,
}

/// The files of every scratch directory, by directory.
type Scratches = Arc<Mutex<BTreeMap<MachinePath, BTreeMap<RepoPath, FileText>>>>;

/// `mutex`, locked: a test that panicked holding it leaves the data as it was.
fn locked<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Where the fake's workspace is.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum FakeWorkspace {
    /// At this directory.
    At(RepoPath),
    /// Nowhere: `find` answers `NoWorkspace`.
    #[default]
    Missing,
    /// In no git repository: `find` and `repository` answer `NoRepository`.
    Unversioned,
}

impl RepositoryFake {
    /// A repository of `files`, its workspace at the root.
    #[must_use]
    pub fn of(files: &[(&str, &str)]) -> Self {
        Self {
            root: MachinePath(FAKE_ROOT.into()),
            files: Arc::new(Mutex::new(
                files
                    .iter()
                    .map(|(path, text)| (RepoPath((*path).into()), FileText((*text).into())))
                    .collect(),
            )),
            workspace: FakeWorkspace::At(RepoPath::default()),
            ..Self::default()
        }
    }

    /// The fake bound to `root`, sharing every file and record with this one.
    fn bound(&self, root: MachinePath) -> Self {
        Self {
            root,
            ..self.clone()
        }
    }

    /// The fake bound to `root`, as the repository the workspace at `workspace` was found in, searched for from there.
    fn found(&self, root: MachinePath, workspace: RepoPath) -> Arc<dyn FoundRepository> {
        Arc::new(Self {
            found_in: workspace,
            ..self.bound(root)
        })
    }

    /// The files of the repository itself, locked.
    fn files(&self) -> std::sync::MutexGuard<'_, BTreeMap<RepoPath, FileText>> {
        locked(&self.files)
    }

    /// The project `path` is, or is within, and the rest of the path below it: one made by `project` and not yet dropped.
    fn scratch_of(&self, path: &MachinePath) -> Option<(MachinePath, String)> {
        let scratches = locked(&self.scratches);
        scratches.keys().find_map(|scratch| {
            let rest = path.0.strip_prefix(&*scratch.0)?;
            let below = rest.strip_prefix('/').or(rest.is_empty().then_some(rest))?;
            Some((scratch.clone(), below.to_owned()))
        })
    }

    /// The files of the repository `root` names: a scratch directory's own, or the fake's.
    fn files_of(&self, root: &MachinePath) -> BTreeMap<RepoPath, FileText> {
        match self.scratch_of(root) {
            Some((scratch, _)) => locked(&self.scratches)
                .get(&scratch)
                .cloned()
                .unwrap_or_default(),
            None => self.files().clone(),
        }
    }
}

impl RepositoryPort for RepositoryFake {
    fn repository(&self, start: &MachinePath) -> Result<MachinePath, RepositoryError> {
        if let Some((scratch, _)) = self.scratch_of(start) {
            return Ok(scratch);
        }
        if self.workspace == FakeWorkspace::Unversioned {
            Err(RepositoryError::NoRepository {
                workspace: start.0.to_string(),
            })
        } else {
            Ok(MachinePath(FAKE_ROOT.into()))
        }
    }

    fn find(&self, start: &MachinePath) -> Result<Arc<dyn FoundRepository>, RepositoryError> {
        if let Some((scratch, below)) = self.scratch_of(start) {
            let files = self.files_of(&scratch);
            let mut directory = below;
            loop {
                let candidate = if directory.is_empty() {
                    "tastemaker.toml".to_owned()
                } else {
                    format!("{directory}/tastemaker.toml")
                };
                if files.contains_key(&RepoPath(candidate.into())) {
                    let workspace = RepoPath(directory.clone().into());
                    return Ok(self.found(scratch, workspace));
                }
                match directory.rsplit_once('/') {
                    Some((above, _)) => directory = above.to_owned(),
                    None if directory.is_empty() => {
                        return Err(RepositoryError::NoWorkspace {
                            start: start.0.to_string(),
                        });
                    }
                    None => directory = String::new(),
                }
            }
        }
        let workspace = match self.workspace.clone() {
            FakeWorkspace::At(workspace) => workspace,
            FakeWorkspace::Missing => {
                return Err(RepositoryError::NoWorkspace {
                    start: start.0.to_string(),
                });
            }
            FakeWorkspace::Unversioned => {
                return Err(RepositoryError::NoRepository {
                    workspace: start.0.to_string(),
                });
            }
        };
        Ok(self.found(MachinePath(FAKE_ROOT.into()), workspace))
    }

    fn open(&self, path: &MachinePath) -> Arc<dyn Repository> {
        Arc::new(self.bound(path.clone()))
    }

    fn skipped(&self) -> SkippedDirectories {
        let mut skipped = self.skipped.clone();
        skipped.push(SkippedDirectory {
            name: DirectoryName(VERSION_CONTROL.into()),
            by: vec![Declarer(
                crate::domain::findings::constants::TASTEMAKER.into(),
            )],
        });
        skipped.sort();
        SkippedDirectories(skipped)
    }

    fn project(&self, root: &MachinePath) -> Result<Box<dyn Repository>, RepositoryError> {
        let mut scratches = locked(&self.scratches);
        if scratches.contains_key(root) {
            return Err(RepositoryError::Write {
                path: root.0.to_string(),
                reason: "it is already there".to_owned(),
            });
        }
        drop(scratches.insert(root.clone(), BTreeMap::new()));
        drop(scratches);
        let _made = self
            .scratched
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(Box::new(ProjectFake {
            repository: Box::new(self.bound(root.clone())),
            scratches: Arc::clone(&self.scratches),
            root: root.clone(),
        }))
    }
}

impl FoundRepository for RepositoryFake {
    fn workspace(&self) -> &RepoPath {
        &self.found_in
    }

    fn start(&self) -> &RepoPath {
        &self.found_in
    }
}

impl Repository for RepositoryFake {
    fn root(&self) -> &MachinePath {
        &self.root
    }

    fn list(&self, under: &RepoPath) -> Result<RepoPaths, RepositoryError> {
        let repository = &self.root;
        Ok(RepoPaths(
            self.files_of(repository)
                .keys()
                // A scratch directory's `.git` is the marker of a repository, never one of its files, as on a disk.
                .filter(|path| {
                    self.scratch_of(repository).is_none() || !path.0.starts_with(".git/")
                })
                .filter(|path| containment(path, under) == Containment::Inside)
                .cloned()
                .collect(),
        ))
    }

    fn walk(&self, under: &RepoPath) -> Result<Walk, RepositoryError> {
        let files = self.list(under)?.0;
        let mut directories = BTreeSet::from([under.clone()]);
        for file in &files {
            let mut rest = &*file.0;
            while let Some((directory, _)) = rest.rsplit_once('/') {
                rest = directory;
                if containment(&RepoPath(directory.into()), under) != Containment::Inside {
                    break;
                }
                let _: bool = directories.insert(RepoPath(directory.into()));
            }
        }
        Ok(Walk {
            files,
            directories: directories.into_iter().collect(),
            links: Vec::new(),
        })
    }

    fn read(&self, path: &RepoPath) -> Result<FileText, RepositoryError> {
        locked(&self.reads).push(path.clone());
        if self.read_failures.contains(path) {
            return Err(RepositoryError::Read {
                path: path.0.to_string(),
                reason: "permission denied".to_owned(),
            });
        }
        self.files_of(&self.root)
            .get(path)
            .cloned()
            .ok_or_else(|| RepositoryError::Read {
                path: path.0.to_string(),
                reason: "no such file".to_owned(),
            })
    }

    fn exists(&self, path: &RepoPath) -> Presence {
        if self
            .files_of(&self.root)
            .keys()
            .any(|file| containment(file, path) == Containment::Inside)
        {
            Presence::Present
        } else {
            Presence::Absent
        }
    }

    fn write(&self, path: &RepoPath, text: &FileText) -> Result<(), RepositoryError> {
        if self.write_failures.contains(path) {
            return Err(RepositoryError::Write {
                path: path.0.to_string(),
                reason: "injected write failure".into(),
            });
        }
        if let Some((scratch, _)) = self.scratch_of(&self.root) {
            drop(
                locked(&self.scratches)
                    .entry(scratch)
                    .or_default()
                    .insert(path.clone(), text.clone()),
            );
            return Ok(());
        }
        let mut files = self.files();
        // A file cannot be written below another file, as on a disk.
        let mut above = &*path.0;
        while let Some((directory, _)) = above.rsplit_once('/') {
            if files.contains_key(&RepoPath(directory.into())) {
                return Err(RepositoryError::Write {
                    path: path.0.to_string(),
                    reason: "a file is in the way".to_owned(),
                });
            }
            above = directory;
        }
        drop(files.insert(path.clone(), text.clone()));
        Ok(())
    }

    fn remove(&self, path: &RepoPath) -> Result<(), RepositoryError> {
        let removed = if let Some((scratch, _)) = self.scratch_of(&self.root) {
            locked(&self.scratches)
                .get_mut(&scratch)
                .and_then(|files| files.remove(path))
        } else {
            self.files().remove(path)
        };
        // A directory is there only while it holds a file, as the memory adapter says it.
        match removed {
            Some(_) => Ok(()),
            None if self.exists(path) == Presence::Absent => Ok(()),
            None => Err(RepositoryError::Write {
                path: path.0.to_string(),
                reason: "the directory holds files".to_owned(),
            }),
        }
    }

    fn stamps(&self, paths: &[RepoPath]) -> crate::domain::files::types::Stamps {
        let found: Vec<Stamp> = {
            let files = self.files_of(&self.root);
            paths
                .iter()
                .map(|path| {
                    files.get(path).map_or_else(
                        || {
                            // A directory's size is how many files it holds, so an added file changes it.
                            let held = files
                                .keys()
                                .filter(|file| {
                                    containment(file, path) == Containment::Inside && *file != path
                                })
                                .count();
                            Stamp {
                                size: crate::domain::files::types::FileSize(
                                    u64::try_from(held).unwrap_or(u64::MAX),
                                ),
                                modified: crate::domain::files::types::ModifiedTime(0),
                            }
                        },
                        |text| Stamp {
                            size: crate::domain::files::types::FileSize(
                                u64::try_from(text.0.len()).unwrap_or(u64::MAX),
                            ),
                            modified: crate::domain::files::types::ModifiedTime(0),
                        },
                    )
                })
                .collect()
        };
        crate::domain::files::types::Stamps(found)
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
