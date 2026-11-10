//! The types of the `file` concept: a file of the repository by its path, what the caller knows of its executable permission, and a file nothing read, with why.

/// A path relative to the root of the repository.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelativePath(pub Box<str>);

/// Why the input edge or a language could not read a file, as it reported.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReadFailure(pub Box<str>);

/// Why a file was not read.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Unreadable {
    /// No linked language reads files like it.
    NoLanguage,
    /// The input edge or a language that reads files like it failed on it.
    Failed(ReadFailure),
}

/// A file nothing read.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Unread {
    /// The file's path, relative to the root.
    pub path: RelativePath,
    /// Why it was not read.
    pub reason: Unreadable,
}

/// What the caller knows of a given file's POSIX executable permission. Unknown never means a known nonexecutable file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExecutablePermission {
    /// The caller supplied no reliable observation.
    #[default]
    Unknown,
    /// The input's permission model has no POSIX executable-bit meaning.
    Unsupported,
    /// The observed mode has no POSIX execute bit (`0o111`).
    PosixNotExecutable,
    /// The observed mode intersects POSIX execute bits (`0o111`).
    PosixExecutable,
}

impl ExecutablePermission {
    /// The engine's permission state of the same observation.
    pub(crate) const fn into_engine(self) -> ryo_values::domain::files::types::ExecutablePermission {
        match self {
            Self::Unknown => ryo_values::domain::files::types::ExecutablePermission::Unknown,
            Self::Unsupported => ryo_values::domain::files::types::ExecutablePermission::Unsupported,
            Self::PosixNotExecutable => {
                ryo_values::domain::files::types::ExecutablePermission::PosixNotExecutable
            }
            Self::PosixExecutable => ryo_values::domain::files::types::ExecutablePermission::PosixExecutable,
        }
    }
}
