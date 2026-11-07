//! The names of the ignore adapter: the file names and marks of a repository on disk.

/// The directory a repository keeps its history in, which is never listed.
#[must_use]
pub(crate) fn git_directory() -> &'static str {
    ".git"
}

/// Why a root that is a file is refused: the walk would list the file under an empty path.
#[must_use]
pub(crate) fn file_root() -> &'static str {
    "it is a file, not a directory"
}

/// What a skip begins with when it names the one directory at a path below the root, not a name at any depth.
#[must_use]
pub(crate) fn exact_skip() -> &'static str {
    "./"
}
