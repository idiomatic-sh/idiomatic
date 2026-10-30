//! The packages port, implemented over local directories: a dependency's `path` is read from the file that names it.

use std::path::{Component, Path, PathBuf};

use super::constants::{
    CURRENT, GIT, HELD, HELD_PACKAGES, MISSING, NOT_UTF8, PARENT, REGISTRY, SEPARATOR,
    SEPARATOR_TEXT,
};
use super::types::HeldPackages;
use crate::domain::files::types::{FileText, Stamp};
use crate::domain::packages::types::{
    Dependency, PackageRoot, PackageSource, SourcePath, VersionText,
};
use crate::domain::paths::types::PathText;
use crate::outbound::packages::errors::PackagesError;
use crate::outbound::packages::interface::PackagesPort;
use crate::outbound::packages::types::{HeldFiles, Located, PackageFile};

/// The path packages adapter, and the packages the run made that it holds in memory, by the directory each is read at.
#[derive(Debug, Default)]
pub(crate) struct PackagesPathAdapter {
    /// The packages held in memory, by their directory.
    pub held: HeldPackages,
    /// How many packages it has held, so each is read at a directory of its own.
    pub made: std::sync::atomic::AtomicU64,
}

impl PackagesPathAdapter {
    /// What `f` makes of the package held at `directory`; none when no package is held there, so the disk answers.
    fn with_held<T>(
        &self,
        directory: &PackageRoot,
        reading: impl FnOnce(&HeldFiles) -> T,
    ) -> Option<T> {
        self.held
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(directory)
            .map(reading)
    }
}

/// The paths of the files of `held` beneath `within`, each relative to it.
fn held_beneath<'a>(
    held: &'a HeldFiles,
    within: &'a PackageFile,
) -> impl Iterator<Item = PathText<'a>> {
    held.0.keys().filter_map(move |file| {
        if within.0.is_empty() {
            Some(PathText(&file.0))
        } else {
            file.0
                .strip_prefix(&*within.0)
                .and_then(|rest| rest.strip_prefix(SEPARATOR))
                .map(PathText)
        }
    })
}

/// `base/child` with `.` and `..` folded lexically, a leading `..` kept.
fn lexical(base: &SourcePath, child: &SourcePath) -> SourcePath {
    let mut parts: Vec<&str> = Vec::new();
    for part in base.0.split(SEPARATOR).chain(child.0.split(SEPARATOR)) {
        if part.is_empty() || part == CURRENT {
            continue;
        }
        if part == PARENT && parts.last().is_some_and(|last| *last != PARENT) {
            let _: Option<&str> = parts.pop();
        } else {
            parts.push(part);
        }
    }
    SourcePath(parts.join(SEPARATOR_TEXT).into())
}

/// `path` with `.` and `..` folded, without reading the disk.
fn folded(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                // `..` above the root stays at the root.
                let _at_root = out.pop();
            }
            Component::CurDir => {}
            other @ (Component::Prefix(_) | Component::RootDir | Component::Normal(_)) => {
                out.push(other.as_os_str());
            }
        }
    }
    out
}

impl PackagesPort for PackagesPathAdapter {
    fn locate(
        &self,
        dependency: &Dependency,
        from: &PackageRoot,
        base: &SourcePath,
    ) -> Result<Located, PackagesError> {
        let path = match &dependency.source {
            PackageSource::Path(path) => path,
            PackageSource::Git(_) => {
                return Err(PackagesError::Unsupported {
                    package: dependency.package.0.to_string(),
                    source_kind: GIT.to_owned(),
                });
            }
            PackageSource::Registry => {
                return Err(PackagesError::Unsupported {
                    package: dependency.package.0.to_string(),
                    source_kind: REGISTRY.to_owned(),
                });
            }
        };
        let directory = folded(&Path::new(&*from.0).join(&*path.0));
        let held = self
            .with_held(&PackageRoot(directory.display().to_string().into()), |_| ())
            .is_some();
        if !held && !directory.is_dir() {
            return Err(PackagesError::Missing {
                package: dependency.package.0.to_string(),
                path: directory.display().to_string(),
            });
        }
        let written = lexical(base, path).0;
        Ok(Located {
            directory: PackageRoot(directory.display().to_string().into()),
            pin: PackageSource::Path(SourcePath(written.clone())),
            base: SourcePath(written),
        })
    }

    fn read(&self, directory: &PackageRoot, file: &PackageFile) -> Result<FileText, PackagesError> {
        let path = Path::new(&*directory.0).join(&*file.0);
        if let Some(held) = self.with_held(directory, |held| held.0.get(file).cloned()) {
            return held.ok_or_else(|| PackagesError::Read {
                path: path.display().to_string(),
                reason: MISSING.to_owned(),
            });
        }
        std::fs::read_to_string(&path)
            .map(|text| FileText(text.into()))
            .map_err(|error| PackagesError::Read {
                path: path.display().to_string(),
                reason: error.to_string(),
            })
    }

    fn offers(&self, directory: &PackageRoot, file: &PackageFile) -> Result<(), PackagesError> {
        if self.with_held(directory, |_| ()).is_some() {
            return self.read(directory, file).map(drop);
        }
        let path = Path::new(&*directory.0).join(&*file.0);
        std::fs::File::open(&path)
            .and_then(|opened| opened.metadata())
            .and_then(|metadata| {
                if metadata.is_file() {
                    Ok(())
                } else {
                    Err(std::io::Error::from(std::io::ErrorKind::IsADirectory))
                }
            })
            .map_err(|error| PackagesError::Read {
                path: path.display().to_string(),
                reason: error.to_string(),
            })
    }

    fn versions(
        &self,
        directory: &PackageRoot,
        within: &PackageFile,
    ) -> Result<crate::outbound::packages::types::Versions, PackagesError> {
        // A held package's versions are the directories its files are in, right beneath `within`.
        if let Some(found) = self.with_held(directory, |held| {
            held_beneath(held, within)
                .filter_map(|rest| rest.0.split_once(SEPARATOR).map(|(name, _)| name))
                .map(|name| VersionText(name.into()))
                .collect::<std::collections::BTreeSet<_>>()
        }) {
            return Ok(crate::outbound::packages::types::Versions(
                found.into_iter().collect(),
            ));
        }
        let path = Path::new(&*directory.0).join(&*within.0);
        let mut found = Vec::new();
        for entry in listed(&path)? {
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                found.push(VersionText(entry.file_name().to_string_lossy().into()));
            }
        }
        found.sort();
        Ok(crate::outbound::packages::types::Versions(found))
    }

    fn files(
        &self,
        directory: &PackageRoot,
        within: &PackageFile,
    ) -> Result<crate::outbound::packages::types::PackageFiles, PackagesError> {
        if let Some(found) = self.with_held(directory, |held| {
            held_beneath(held, within)
                .map(|rest| PackageFile(rest.0.into()))
                .collect::<Vec<_>>()
        }) {
            return Ok(crate::outbound::packages::types::PackageFiles(found));
        }
        let root = Path::new(&*directory.0).join(&*within.0);
        let mut found = Vec::new();
        let mut pending = vec![(root, String::new())];
        while let Some((current, prefix)) = pending.pop() {
            for entry in listed(&current)? {
                let name = entry.file_name().to_string_lossy().into_owned();
                let relative = if prefix.is_empty() {
                    name
                } else {
                    format!("{prefix}{SEPARATOR_TEXT}{name}")
                };
                if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                    pending.push((entry.path(), relative));
                } else {
                    found.push(PackageFile(relative.into()));
                }
            }
        }
        found.sort();
        Ok(crate::outbound::packages::types::PackageFiles(found))
    }

    fn stamps(
        &self,
        files: &[crate::outbound::packages::types::PackagedFile],
    ) -> crate::domain::files::types::Stamps {
        let found: Vec<Stamp> = {
            files
                .iter()
                .map(|packaged| {
                    // A held file is as long as its text, and was never written.
                    if let Some(held) = self.with_held(&packaged.directory, |held| {
                        held.0.get(&packaged.file).map(|text| Stamp {
                            size: crate::domain::files::types::FileSize(
                                u64::try_from(text.0.len()).unwrap_or(u64::MAX),
                            ),
                            modified: crate::domain::files::types::ModifiedTime(0),
                        })
                    }) {
                        return held.unwrap_or_default();
                    }
                    let stamped = std::fs::metadata(
                        Path::new(&*packaged.directory.0).join(&*packaged.file.0),
                    )
                    .and_then(|meta| {
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

    fn hold(&self, files: HeldFiles) -> PackageRoot {
        let mut held = self
            .held
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // The number is taken under the lock, so holds insert in the order of their numbers whatever the threads do: a number taken
        // before the lock could be inserted after the `HELD_PACKAGES` that follow it, and never let go.
        let number = self.made.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let directory = PackageRoot(format!("{HELD}{number}").into());
        drop(held.insert(directory.clone(), files));
        // The numbers are taken in order, so the one held longest is the one `HELD_PACKAGES` numbers back: the directories'
        // text does not sort by number.
        if let Some(oldest) = number.checked_sub(u64::try_from(HELD_PACKAGES).unwrap_or(u64::MAX)) {
            drop(held.remove(&PackageRoot(format!("{HELD}{oldest}").into())));
        }
        directory
    }
}

/// Each entry of the directory at `path`, its kind read and its name UTF-8: none when there is no such directory, and a
/// failure for one that cannot be listed, an entry whose kind cannot be read and a name that is not UTF-8, which would
/// otherwise be left out unseen.
fn listed(path: &Path) -> Result<Vec<std::fs::DirEntry>, PackagesError> {
    let failed = |at: &Path, reason: &dyn std::fmt::Display| PackagesError::Read {
        path: at.display().to_string(),
        reason: reason.to_string(),
    };
    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(failed(path, &error)),
    };
    entries
        .map(|entry| {
            let entry = entry.map_err(|error| failed(path, &error))?;
            let _kind: std::fs::FileType = entry
                .file_type()
                .map_err(|error| failed(&entry.path(), &error))?;
            if entry.file_name().to_str().is_none() {
                return Err(failed(&entry.path(), &NOT_UTF8));
            }
            Ok(entry)
        })
        .collect()
}
