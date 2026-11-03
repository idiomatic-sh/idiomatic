//! The corpus port, implemented with the `ignore` crate: the files on disk that `.gitignore` does
//! not exclude, found by a parallel walk.

use crate::outbound::corpus::types::{Inventory, PathStat, SkippedDirectories};
use std::io::Read as _;
use std::sync::mpsc;

use ignore::{WalkBuilder, WalkState};

use super::constants::{exact_skip, file_root, git_directory};
use super::types::{BytesRead, Stability, Stamp};
use crate::outbound::corpus::errors::CorpusError;
use crate::outbound::corpus::interface::CorpusPort;
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_values::domain::files::service::with_separator;
use ryo_values::domain::files::types::{
    CorpusEntry, CorpusFile, CorpusListing, FileBytes, FileSize, Nanoseconds, NativeSeparator,
    PlatformPath,
};

/// The ignore corpus adapter: which files it lists, and the files given as text it reads in place of the disk's.
#[derive(Debug)]
pub(crate) struct CorpusIgnoreAdapter {
    /// The repository's root directory.
    root: RootPath,
    /// Walked, or the files the caller listed.
    listing: CorpusListing,
    /// Files given as text: listed beside the others, and read as given, also where the disk has a file of that path.
    beside: Vec<CorpusFile>,
    /// Directories a walk never goes into, at any depth.
    skipped: SkippedDirectories,
}

impl CorpusIgnoreAdapter {
    /// An adapter over the repository at `root` that lists the files `listing` says, reads those of `beside` as given, and walks
    /// into no directory of `skipped`.
    pub(crate) const fn new(
        root: RootPath,
        listing: CorpusListing,
        beside: Vec<CorpusFile>,
        skipped: SkippedDirectories,
    ) -> Self {
        Self {
            root,
            listing,
            beside,
            skipped,
        }
    }
}

/// When a file given as text was written: it is dated by what it holds, so an edit changes the date.
fn given_date(bytes: &FileBytes) -> Nanoseconds {
    let mut hasher = rustc_hash::FxHasher::default();
    std::hash::Hasher::write(&mut hasher, &bytes.0);
    Nanoseconds(std::hash::Hasher::finish(&hasher) | 1)
}

/// The files below `root` the repository does not ignore, found by a parallel walk.
#[expect(
    clippy::filetype_is_file,
    reason = "only regular files are listed: a symbolic link may lead to a directory, and a FIFO or a device would block or never end when read"
)]
fn walked(root: &RootPath, skipped: &SkippedDirectories) -> Result<Vec<CorpusEntry>, CorpusError> {
    // A root that is a file is no repository: the walk would list the file under an empty path.
    if std::fs::metadata(&*root.0).is_ok_and(|meta| !meta.is_dir()) {
        return Err(CorpusError::Unreadable {
            path: root.0.to_string(),
            reason: file_root().to_owned(),
        });
    }
    let skipped = skipped.clone();
    let base = std::path::PathBuf::from(&*root.0);
    let (sender, receiver) = mpsc::channel::<Result<CorpusEntry, String>>();
    let mut builder = WalkBuilder::new(&*root.0);
    let _: &mut WalkBuilder = builder
        .hidden(false)
        .require_git(false)
        // Only the repository's own ignore files apply, never a user's global excludes, so every machine walks the same files.
        .git_global(false)
        .filter_entry(move |entry| {
            let name = entry.file_name();
            if name == git_directory() {
                return false;
            }
            if !entry.file_type().is_some_and(|kind| kind.is_dir()) {
                return true;
            }
            // `./a/b` is the directory at that path below the root; a name is a directory of that name at any depth.
            let below = entry.path().strip_prefix(&base);
            !skipped
                .0
                .iter()
                .any(|skip| match skip.strip_prefix(exact_skip()) {
                    Some(path) => below
                        .as_ref()
                        .is_ok_and(|relative| *relative == std::path::Path::new(path)),
                    None => name == &**skip,
                })
        });
    builder.build_parallel().run(|| {
        let sender = sender.clone();
        Box::new(move |result| {
            let sent = match result {
                Err(error) => sender.send(Err(error.to_string())),
                Ok(entry) if entry.file_type().is_some_and(|kind| kind.is_file()) => {
                    let Ok(meta) = entry.metadata() else {
                        return WalkState::Continue;
                    };
                    let Ok(relative) = entry.path().strip_prefix(&*root.0) else {
                        return WalkState::Continue;
                    };
                    sender.send(Ok(CorpusEntry {
                        executable: executable(&meta),
                        path: with_separator(
                            PlatformPath(&relative.to_string_lossy()),
                            NativeSeparator(std::path::MAIN_SEPARATOR),
                        ),
                        size: FileSize(meta.len()),
                        modified: modified(&meta),
                    }))
                }
                Ok(_) => return WalkState::Continue,
            };
            if sent.is_err() {
                WalkState::Quit
            } else {
                WalkState::Continue
            }
        })
    });
    drop(sender);
    receiver
        .into_iter()
        .collect::<Result<Vec<_>, String>>()
        .map_err(|reason| unreadable(root, &reason))
}

/// The files the caller listed that are files below `root`, each with what the system says of it; a listed path that is not
/// there, or is not a file, is not listed.
///
/// # Errors
///
/// `Unreadable` when the system cannot say what a listed path is.
fn listed(
    root: &RootPath,
    paths: &[ryo_language::domain::files::types::FilePath],
) -> Result<Vec<CorpusEntry>, CorpusError> {
    use rayon::prelude::*;
    let base = std::path::Path::new(&*root.0);
    let found = paths
        .par_iter()
        .map(|path| match std::fs::metadata(base.join(&*path.0)) {
            Ok(meta) => Ok(meta.is_file().then(|| CorpusEntry {
                executable: executable(&meta),
                path: path.clone(),
                size: FileSize(meta.len()),
                modified: modified(&meta),
            })),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(unreadable(root, &error)),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(found.into_iter().flatten().collect())
}

/// The refusal of a file below `root` that could not be read, saying why.
fn unreadable(root: &RootPath, reason: &dyn std::fmt::Display) -> CorpusError {
    CorpusError::Unreadable {
        path: root.0.to_string(),
        reason: reason.to_string(),
    }
}

/// The refusal of a file that could not be read, naming the file, and saying why.
fn unreadable_file(entry: &CorpusEntry, reason: &dyn std::fmt::Display) -> CorpusError {
    CorpusError::Unreadable {
        path: entry.path.0.to_string(),
        reason: reason.to_string(),
    }
}

/// When the file was last written, in nanoseconds since 1970; zero when the system does not say.
fn modified(meta: &std::fs::Metadata) -> Nanoseconds {
    Nanoseconds(
        meta.modified()
            .ok() // ignore: idiomatic.arch/no-ok-on-result - a system that keeps no modification time says nothing, which the record keeps as zero
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok()) // ignore: idiomatic.arch/no-ok-on-result - a time before 1970 has no count since it, which the record keeps as zero
            .map_or(0, |since| {
                u64::try_from(since.as_nanos()).unwrap_or(u64::MAX)
            }),
    )
}

/// What the system says of the open file.
fn stamp(file: &std::fs::File) -> std::io::Result<Stamp> {
    let meta = file.metadata()?;
    Ok(Stamp {
        size: FileSize(meta.len()),
        written: modified(&meta),
        changed: changed(&meta),
    })
}

/// When the file's inode last changed, in nanoseconds since 1970.
#[cfg(unix)]
fn changed(meta: &std::fs::Metadata) -> Nanoseconds {
    use std::os::unix::fs::MetadataExt as _;
    Nanoseconds(
        u64::try_from(meta.ctime())
            .unwrap_or(0)
            .saturating_mul(1_000_000_000)
            .saturating_add(u64::try_from(meta.ctime_nsec()).unwrap_or(0)),
    )
}

/// When the file's inode last changed: the system does not say.
#[cfg(not(unix))]
fn changed(_meta: &std::fs::Metadata) -> Nanoseconds {
    Nanoseconds(0)
}

/// Whether a read is stable: the file says the same after it as before, and the bytes read are the size it said.
pub(super) fn agree(before: &Stamp, after: &Stamp, read: BytesRead) -> Stability {
    if before == after && read.0 == before.size.0 {
        Stability::Stable
    } else {
        Stability::Unstable
    }
}

impl CorpusPort for CorpusIgnoreAdapter {
    fn inventory(&self) -> Result<Inventory, CorpusError> {
        let root = &self.root;
        let mut entries = match &self.listing {
            CorpusListing::Walked => walked(root, &self.skipped)?,
            CorpusListing::Listed(paths) => listed(root, paths)?,
        };
        if !self.beside.is_empty() {
            entries.retain(|entry| self.beside.iter().all(|given| given.path != entry.path));
            entries.extend(self.beside.iter().map(|given| CorpusEntry {
                executable: given.executable,
                path: given.path.clone(),
                size: FileSize(u64::try_from(given.bytes.0.len()).unwrap_or(u64::MAX)),
                modified: given_date(&given.bytes),
            }));
        }
        Ok(Inventory(entries))
    }

    fn read(&self, entry: &CorpusEntry, into: &mut FileBytes) -> Result<(), CorpusError> {
        let root = &self.root;
        if let Some(given) = self.beside.iter().find(|given| given.path == entry.path) {
            into.0.clear();
            into.0.extend_from_slice(&given.bytes.0);
            return Ok(());
        }
        let into = &mut into.0;
        let path = std::path::Path::new(&*root.0).join(&*entry.path.0);
        // A read is stable when the open file says the same before and after it and the bytes read are as many as it said; one that is
        // not is read once more, then refused.
        for _ in 0_i32..2_i32 {
            into.clear();
            into.reserve(usize::try_from(entry.size.0).unwrap_or(0).saturating_add(1));
            let file =
                std::fs::File::open(&path).map_err(|error| unreadable_file(entry, &error))?;
            let before = stamp(&file).map_err(|error| unreadable_file(entry, &error))?;
            if before.size.0 > u64::from(u32::MAX) {
                return Err(CorpusError::TooLarge {
                    path: entry.path.0.to_string(),
                    size: before.size,
                });
            }
            // One byte past the size the file said is enough to see that it grew while it was read.
            let read = std::io::Read::take(&file, before.size.0.saturating_add(1))
                .read_to_end(into)
                .map_err(|error| unreadable_file(entry, &error))?;
            let after = stamp(&file).map_err(|error| unreadable_file(entry, &error))?;
            if agree(
                &before,
                &after,
                BytesRead(u64::try_from(read).unwrap_or(u64::MAX)),
            ) == Stability::Stable
            {
                return Ok(());
            }
        }
        into.clear();
        Err(CorpusError::Unstable {
            path: entry.path.0.to_string(),
        })
    }

    fn stat(
        &self,
        path: &ryo_language::domain::files::types::FilePath,
    ) -> Result<PathStat, CorpusError> {
        if let Some(given) = self.beside.iter().find(|given| given.path == *path) {
            return Ok(PathStat::Found(CorpusEntry {
                executable: given.executable,
                path: given.path.clone(),
                size: FileSize(u64::try_from(given.bytes.0.len()).unwrap_or(u64::MAX)),
                modified: given_date(&given.bytes),
            }));
        }
        Ok(listed(&self.root, std::slice::from_ref(path))?
            .pop()
            .map_or(PathStat::Missing, PathStat::Found))
    }
}

/// Observe only the platform's actual permission model; reference WSL policy stays at its edge.
#[cfg(unix)]
fn executable(
    metadata: &std::fs::Metadata,
) -> ryo_values::domain::files::types::ExecutablePermission {
    use std::os::unix::fs::PermissionsExt as _;
    ryo_values::domain::files::types::ExecutablePermission::Posix(
        metadata.permissions().mode() & 0o111 != 0,
    )
}
#[cfg(not(unix))]
fn executable(_: &std::fs::Metadata) -> ryo_values::domain::files::types::ExecutablePermission {
    ryo_values::domain::files::types::ExecutablePermission::Unsupported
}
