// ignore: idiomatic.arch/file-length - the adapter is the in-memory twin of the disk adapter: it answers every method of the repository port over one tree type, git's ignore files included, as a disk would; its plain types are in types.rs and its names in constants.rs, and what is left would split only by port method, which the adapter layout refuses.
//! The repository port in memory: each scratch directory a tree of texts, used as the disk adapter uses a disk.

use std::collections::BTreeMap;
use std::io::{Error, ErrorKind};
use std::sync::{Arc, Mutex, PoisonError};

use ignore::gitignore::{Gitignore, GitignoreBuilder};

use super::constants::{
    ALREADY_THERE, GIT, GIT_IGNORE, IGNORE, MISSING_FILE, TASTEMAKER, TOO_MANY, TREES,
    WORKSPACE_FILE,
};
use super::types::{
    Entering, HeldFile, IgnoreFile, Ignoring, Kind, Matchers, RepositoryMemoryProject,
    RepositoryMemoryRepository, ScratchPlace, Tree, Trees, Writes,
};
use crate::domain::files::types::{
    Declarer, DirectoryName, FileSize, FileText, MachinePath, ModifiedTime, SkippedDirectories,
    SkippedDirectory, Stamp,
};
use crate::domain::paths::service::{ancestors, containment, join, name, parent, relative};
use crate::domain::paths::types::{Containment, PathText, RepoPath};
use crate::outbound::repository::errors::RepositoryError;
use crate::outbound::repository::interface::{FoundRepository, Repository, RepositoryPort};
use crate::outbound::repository::types::{Presence, RepoPaths, Walk};

/// The memory repository adapter: its trees, and the directories a walk skips.
#[derive(Debug)]
pub(crate) struct RepositoryMemoryAdapter {
    /// The directories a walk never goes into, at any depth, from the linked languages.
    pub skipped: Vec<SkippedDirectory>,
    /// The store's directory, never walked.
    pub store: DirectoryName,
    /// The trees, by their scratch directory.
    pub trees: Arc<Trees>,
}

/// What a read of a file that is not there says, in the system's own words, as a read from the disk says them.
fn missing() -> Error {
    Error::from_raw_os_error(MISSING_FILE)
}

/// `path` beneath `directory`; the directory itself when it climbs above it.
fn beneath(directory: &RepoPath, path: PathText<'_>) -> RepoPath {
    join(directory, path).unwrap_or_else(|| directory.clone())
}

/// Why a write or a removal of `path` is refused.
fn unwritten(path: &RepoPath, reason: &Error) -> RepositoryError {
    RepositoryError::Write {
        path: path.0.to_string(),
        reason: reason.to_string(),
    }
}

/// Whether `tree` holds a file at `path`, or a file beneath it.
fn holds(tree: &Tree, path: &RepoPath) -> Presence {
    let after = tree.files.range(path.clone()..).map(|(file, _)| file);
    let inside = |file: &RepoPath| containment(file, path) == Containment::Inside;
    let mut below = after.take_while(|file| file.0.starts_with(&*path.0));
    if below.any(inside) {
        Presence::Present
    } else {
        Presence::Absent
    }
}

/// The ignore file `file` of `directory` compiled, when `tree` holds it, leaving out a line that does not compile.
fn matcher(tree: &Tree, directory: &RepoPath, file: IgnoreFile) -> Option<Gitignore> {
    let held = tree.files.get(&beneath(directory, PathText(file.0)))?;
    let mut builder = GitignoreBuilder::new(format!("/{}", directory.0));
    for line in held.text.0.lines() {
        drop(builder.add_line(None, line));
    }
    builder.build().ok() // ignore: idiomatic.arch/no-ok-on-result - an ignore file whose patterns do not build ignores nothing, as git skips a pattern it cannot read
}

/// Whether `path` is ignored as `ignore` decides it: by the nearest `.ignore` that matches, else by the nearest
/// `.gitignore` that matches up to the nearest directory holding `.git`.
fn ignored(tree: &Tree, path: &RepoPath, kind: Kind, matchers: &mut Matchers) -> Ignoring {
    let absolute = format!("/{}", path.0);
    let mut matched = |directory: &RepoPath, file: IgnoreFile| {
        let entry = matchers.0.entry(beneath(directory, PathText(file.0)));
        let compiled = entry.or_insert_with(|| matcher(tree, directory, file));
        let is_directory = kind == Kind::Directory;
        let matched =
            |gitignore: &Gitignore| gitignore.matched(&absolute, is_directory).map(|_| ());
        compiled.as_ref().map_or(ignore::Match::None, matched)
    };
    let (mut by_ignore, mut by_git) = (ignore::Match::None, ignore::Match::None);
    let mut saw_git = Presence::Absent;
    for directory in ancestors(path) {
        if by_ignore.is_none() {
            by_ignore = matched(&directory, IgnoreFile(IGNORE));
        }
        if saw_git == Presence::Absent && by_git.is_none() {
            by_git = matched(&directory, IgnoreFile(GIT_IGNORE));
        }
        if saw_git == Presence::Absent {
            saw_git = holds(tree, &beneath(&directory, PathText(GIT)));
        }
    }
    if by_ignore.is_ignore() || (by_ignore.is_none() && by_git.is_ignore()) {
        Ignoring::Ignored
    } else {
        Ignoring::Kept
    }
}

/// The stamp of `target` in `tree`: a file's size and writes; a directory's size is how many files it holds.
fn stamp(tree: &Tree, target: &RepoPath) -> Stamp {
    let count = |files| FileSize(u64::try_from(files).unwrap_or(u64::MAX));
    match tree.files.get(target) {
        Some(held) => Stamp {
            size: count(held.text.0.len()),
            modified: ModifiedTime(held.writes.0),
        },
        None if holds(tree, target) == Presence::Present => {
            let held = tree.files.keys();
            let files = held.filter(|file| parent(file).as_ref() == Some(target));
            Stamp {
                size: count(files.count()),
                modified: ModifiedTime::default(),
            }
        }
        None => Stamp::default(),
    }
}

impl RepositoryMemoryAdapter {
    /// The directory `root`, bound, walked as this adapter walks.
    fn bound(&self, root: MachinePath) -> RepositoryMemoryRepository {
        RepositoryMemoryRepository {
            root,
            skipped: self.skipped.clone(),
            store: self.store.clone(),
            trees: Arc::clone(&self.trees),
            workspace: RepoPath::default(),
            start: RepoPath::default(),
        }
    }
}

impl RepositoryMemoryRepository {
    /// What `each` asks of the tree the root is in, where the root and `path` are in it; `missing` when there is none.
    fn with<T>(
        &self,
        path: &RepoPath,
        missing: impl FnOnce() -> T,
        each: impl FnOnce(&mut Tree, &ScratchPlace) -> T,
    ) -> T {
        let trees = self.trees.0.lock().unwrap_or_else(PoisonError::into_inner);
        let found = trees.iter().find_map(|(scratch, tree)| {
            let rest = self.root.0.strip_prefix(&*scratch.0)?;
            let below = rest.strip_prefix('/').or(rest.is_empty().then_some(rest))?;
            Some((scratch.clone(), PathText(below), Arc::clone(tree)))
        });
        drop(trees);
        let Some((scratch, below, tree)) = found else {
            return missing();
        };
        let below = beneath(&RepoPath::default(), below);
        let target = beneath(&below, PathText(&path.0));
        let at = ScratchPlace {
            scratch,
            below,
            target,
        };
        each(
            &mut tree.lock().unwrap_or_else(PoisonError::into_inner),
            &at,
        )
    }

    /// Whether a walk of `tree` goes into the directory `path`: not `.git`, the store, a skipped or an ignored one.
    fn enters(&self, tree: &Tree, path: &RepoPath, matchers: &mut Matchers) -> Entering {
        let named = name(path).0;
        let skipped = &*named == GIT
            || *named == *self.store.0
            || self.skipped.iter().any(|skip| named == skip.name.0);
        if skipped || ignored(tree, path, Kind::Directory, matchers) == Ignoring::Ignored {
            Entering::Skips
        } else {
            Entering::Enters
        }
    }
}

impl RepositoryPort for RepositoryMemoryAdapter {
    fn repository(&self, start: &MachinePath) -> Result<MachinePath, RepositoryError> {
        self.bound(start.clone()).with(
            &RepoPath::default(),
            || {
                Err(RepositoryError::NoRepository {
                    workspace: start.0.to_string(),
                })
            },
            |tree, at| {
                if holds(tree, &RepoPath(GIT.into())) == Presence::Absent {
                    return Err(RepositoryError::NoRepository {
                        workspace: start.0.to_string(),
                    });
                }
                Ok(at.scratch.clone())
            },
        )
    }

    fn find(&self, start: &MachinePath) -> Result<Arc<dyn FoundRepository>, RepositoryError> {
        let no_workspace = || RepositoryError::NoWorkspace {
            start: start.0.to_string(),
        };
        self.bound(start.clone()).with(
            &RepoPath::default(),
            || Err(no_workspace()),
            |tree, at| {
                if holds(tree, &RepoPath(GIT.into())) == Presence::Absent {
                    return Err(RepositoryError::NoRepository {
                        workspace: start.0.to_string(),
                    });
                }
                let marked = |directory: &RepoPath| {
                    tree.files
                        .contains_key(&beneath(directory, PathText(WORKSPACE_FILE)))
                };
                let mut above = std::iter::once(at.below.clone()).chain(ancestors(&at.below));
                above
                    .find(marked)
                    .map(|workspace| -> Arc<dyn FoundRepository> {
                        Arc::new(RepositoryMemoryRepository {
                            workspace,
                            start: at.below.clone(),
                            ..self.bound(at.scratch.clone())
                        })
                    })
                    .ok_or_else(no_workspace)
            },
        )
    }

    fn open(&self, path: &MachinePath) -> Arc<dyn Repository> {
        Arc::new(self.bound(path.clone()))
    }

    fn skipped(&self) -> SkippedDirectories {
        let mut skipped = self.skipped.clone();
        for name in [DirectoryName(GIT.into()), self.store.clone()] {
            let by = vec![Declarer(TASTEMAKER.into())];
            skipped.push(SkippedDirectory { name, by });
        }
        skipped.sort();
        SkippedDirectories(skipped)
    }

    fn project(&self, root: &MachinePath) -> Result<Box<dyn Repository>, RepositoryError> {
        let mut trees = self.trees.0.lock().unwrap_or_else(PoisonError::into_inner);
        if trees.contains_key(root) {
            return Err(RepositoryError::Write {
                path: root.0.to_string(),
                reason: ALREADY_THERE.into(),
            });
        }
        if trees.len() >= TREES {
            return Err(RepositoryError::Write {
                path: root.0.to_string(),
                reason: TOO_MANY.into(),
            });
        }
        drop(trees.insert(root.clone(), Arc::new(Mutex::new(Tree::default()))));
        drop(trees);
        Ok(Box::new(RepositoryMemoryProject {
            repository: self.bound(root.clone()),
        }))
    }
}

impl FoundRepository for RepositoryMemoryRepository {
    fn workspace(&self) -> &RepoPath {
        &self.workspace
    }

    fn start(&self) -> &RepoPath {
        &self.start
    }
}

impl Repository for RepositoryMemoryRepository {
    fn root(&self) -> &MachinePath {
        &self.root
    }

    fn list(&self, under: &RepoPath) -> Result<RepoPaths, RepositoryError> {
        Ok(RepoPaths(self.walk(under)?.files))
    }

    fn walk(&self, under: &RepoPath) -> Result<Walk, RepositoryError> {
        Ok(self.with(under, Walk::default, |tree, at| {
            let start = &at.target;
            // A path relative to the repository walked, as the disk adapter answers it.
            let answered = |path: &RepoPath| {
                let rest = relative(path, &at.below);
                rest.map_or_else(|| path.clone(), |rest| RepoPath(rest.0.into()))
            };
            if tree.files.contains_key(start) {
                let files = vec![answered(start)];
                return Walk {
                    files,
                    ..Walk::default()
                };
            }
            if holds(tree, start) == Presence::Absent {
                return Walk::default();
            }
            // Each directory between the start and a file is entered or refused once, the outermost first.
            let (mut matchers, mut files) = (Matchers::default(), Vec::new());
            let mut decided = BTreeMap::from([(start.clone(), Entering::Enters)]);
            for path in tree
                .files
                .keys()
                .filter(|path| containment(path, start) == Containment::Inside)
            {
                let between = ancestors(path)
                    .into_iter()
                    .take_while(|directory| directory != start);
                let entered = between
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .all(|directory| {
                        let known = decided.get(&directory).copied();
                        let entering =
                            known.unwrap_or_else(|| self.enters(tree, &directory, &mut matchers));
                        let _: Option<Entering> = decided.insert(directory, entering);
                        entering == Entering::Enters
                    });
                let file = entered && &*name(path).0 != GIT;
                if file && ignored(tree, path, Kind::File, &mut matchers) == Ignoring::Kept {
                    files.push(answered(path));
                }
            }
            let entered = decided
                .iter()
                .filter(|(_, entering)| **entering == Entering::Enters);
            let directories = entered.map(|(directory, _)| answered(directory)).collect();
            Walk {
                files,
                directories,
                links: Vec::new(),
            }
        }))
    }

    fn read(&self, path: &RepoPath) -> Result<FileText, RepositoryError> {
        let refused = || RepositoryError::Read {
            path: path.0.to_string(),
            reason: missing().to_string(),
        };
        self.with(
            path,
            || Err(refused()),
            |tree, at| {
                let held = tree.files.get(&at.target);
                held.map(|held| held.text.clone()).ok_or_else(refused)
            },
        )
    }

    fn exists(&self, path: &RepoPath) -> Presence {
        self.with(
            path,
            || Presence::Absent,
            |tree, at| holds(tree, &at.target),
        )
    }

    fn write(&self, path: &RepoPath, text: &FileText) -> Result<(), RepositoryError> {
        let refused = |reason| unwritten(path, &reason);
        self.with(
            path,
            || Err(refused(missing())),
            |tree, at| {
                let target = &at.target;
                // A file cannot be written below another file, nor where a directory is, as on a disk.
                let mut above = ancestors(target).into_iter();
                let under_file = above.any(|directory| tree.files.contains_key(&directory));
                let file = tree.files.contains_key(target);
                let on_directory = !file && holds(tree, target) == Presence::Present;
                if under_file || on_directory || target.0.is_empty() {
                    return Err(refused(Error::from(ErrorKind::NotADirectory)));
                }
                let writes = tree.files.get(target).map_or(0, |held| held.writes.0);
                let held = HeldFile {
                    text: text.clone(),
                    writes: Writes(writes.saturating_add(1)),
                };
                drop(tree.files.insert(target.clone(), held));
                Ok(())
            },
        )
    }

    fn remove(&self, path: &RepoPath) -> Result<(), RepositoryError> {
        let refused = |reason| unwritten(path, &reason);
        self.with(
            path,
            || Err(refused(missing())),
            |tree, at| {
                if tree.files.remove(&at.target).is_some() {
                    return Ok(());
                }
                // A directory is there only while it holds a file: one that holds one cannot be removed, and one that holds
                // none is not there.
                Err(refused(if holds(tree, &at.target) == Presence::Present {
                    Error::from(ErrorKind::DirectoryNotEmpty)
                } else {
                    missing()
                }))
            },
        )
    }

    fn stamps(&self, paths: &[RepoPath]) -> crate::domain::files::types::Stamps {
        crate::domain::files::types::Stamps(self.with(
            &RepoPath::default(),
            || vec![Stamp::default(); paths.len()],
            |tree, at| {
                let stamped = paths
                    .iter()
                    .map(|path| stamp(tree, &beneath(&at.below, PathText(&path.0))));
                stamped.collect()
            },
        ))
    }
}

impl Repository for RepositoryMemoryProject {
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
