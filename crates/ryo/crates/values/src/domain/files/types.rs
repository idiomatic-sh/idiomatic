//! The types of the `files` concept: the files of a run and the globs that name them. Plain data, public fields and derives, with no constructor and no inherent impl.

use std::sync::Arc;

use ryo_language::domain::files::types::FilePath;

/// What an input edge knows of POSIX executable permission semantics.
/// Unavailable observations never mean a known nonexecutable file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExecutablePermission {
    /// The edge supplied no reliable observation.
    #[default]
    Unknown,
    /// The input's permission model has no POSIX executable-bit meaning.
    Unsupported,
    /// The observed mode intersects POSIX execute bits (`0o111`).
    Posix(bool),
}

/// One file of the repository, found but not read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CorpusEntry {
    /// Edge-observed permission state, independent of source bytes and modification time.
    pub executable: ExecutablePermission,
    /// The path, relative to the root, with `/` between directories.
    pub path: FilePath,
    /// The file's size in bytes.
    pub size: FileSize,
    /// When the file was last written; with the size, the token that says it is as it was.
    pub modified: Nanoseconds,
}

/// A listed input has either real observed metadata or a truthful per-path discovery failure.
/// Unreadable inputs never carry fabricated file sizes, timestamps or source bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CorpusInput {
    /// A real file whose metadata the edge observed.
    Available(CorpusEntry),
    /// A primary inventoried file failed an earlier read, retaining its real observed metadata.
    /// This is not a provider refusal or an unrelated consulted dependency.
    ReadFailed {
        /// The metadata that was actually observed before the read failed.
        entry: CorpusEntry,
        /// The original input-edge diagnostic detail.
        reason: InputReadFailure,
    },
    /// An explicitly requested file could not be observed on disk.
    Unreadable {
        /// The exact requested path, relative to the root.
        path: FilePath,
        /// The edge's original diagnostic detail.
        reason: InputReadFailure,
    },
}

/// The original detail of an input discovery/read failure, without a provider or lint classification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputReadFailure(pub Box<str>);

/// One file of the repository with its bytes: what a fake keeps and hands out on `read`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CorpusFile {
    /// Edge-observed permission state, independent of source bytes and modification time.
    pub executable: ExecutablePermission,
    /// The path, relative to the root, with `/` between directories.
    pub path: FilePath,
    /// The file's bytes.
    pub bytes: FileBytes,
}

/// Which files of the root a corpus lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CorpusListing {
    /// The files below the root the repository does not ignore, found by walking it.
    Walked,
    /// These files, as the caller listed them: nothing is walked, and no other file is read.
    Listed(Vec<FilePath>),
}

/// The bytes of one file as the corpus reads them: the buffer a caller keeps for all its files, which a read replaces.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileBytes(pub Vec<u8>);

/// A moment as the file system dates it, in nanoseconds since 1970; zero when it does not say.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Nanoseconds(pub u64);

/// How many bytes a file holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileSize(pub u64);

/// A path as a platform writes it, borrowed: its directories may be split by the platform's separator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlatformPath<'a>(pub &'a str);

/// The character a platform splits the directories of a path with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeSeparator(pub char);

/// The number a file had in a run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileNumber(pub u32);

/// A glob that names files.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Glob(pub Box<str>);

/// The path a matcher is asked about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatchCandidate<'a>(pub &'a str);

/// The globs of a matcher that matched a path, by their place among the globs it was made of.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Hits(pub Vec<usize>);

/// Many globs made ready to match a path against all of them at once. A matcher belongs to no
/// worker: any thread may call it, and each brings its own `Hits`.
#[derive(Clone)]
pub struct GlobsMatcher {
    /// Finds the globs that match a path, in `Hits`.
    pub matches: Arc<dyn Matching>,
}

impl std::fmt::Debug for GlobsMatcher {
    /// The matcher for a debug print: what it matches with is a closure or a library's matcher, which has no `Debug`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

/// What finds the globs that match a path.
pub trait Matching: Send + Sync {
    /// Puts into `hits` the globs that match the path.
    fn find(&self, path: MatchCandidate<'_>, hits: &mut Hits);
}

impl<F: Fn(MatchCandidate<'_>, &mut Hits) + Send + Sync> Matching for F {
    fn find(&self, path: MatchCandidate<'_>, hits: &mut Hits) {
        self(path, hits);
    }
}
