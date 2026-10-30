//! The names of the `links` concept: the files a package is, and the text a finding about the links shows.

/// The ending of a Go file.
pub(crate) const GO: &str = ".go";

/// The ending of a Go test file, which is no part of the package an import names.
pub(crate) const TEST: &str = "_test.go";

/// What separates the parts of a path, and of an import path.
pub(crate) const SEPARATOR: char = '/';

/// The directory at the root of the repository.
pub(crate) const ROOT: &str = crate::domain::modules::constants::ROOT;

/// Why an import that names a package of the file's module no directory is makes the relation read in part, after the path.
pub(crate) const UNRESOLVED_CAUSE: &str =
    "names a package of the module that no directory of Go files is";

/// Why the imports of a file whose module's `go.mod` was refused make the relation read in part, after the manifest.
pub(crate) const UNREAD_CAUSE: &str =
    "declares no module the imports of its files could be linked to";
