//! The names of the `links` concept: the files a specifier may name, and the text a finding about the links shows.

/// The endings a specifier that names no file is tried with, in order.
pub(crate) const EXTENSIONS: [&str; 4] = [
    crate::domain::language::constants::PLAIN_EXTENSION,
    crate::domain::language::constants::MODULE_EXTENSION,
    crate::domain::language::constants::SCRIPT_EXTENSION,
    crate::domain::language::constants::JSX_EXTENSION,
];

/// The file a directory a specifier names is, without its ending.
pub(crate) const INDEX: &str = "index";

/// What separates the parts of a path.
pub(crate) const SEPARATOR: char = '/';

/// A part of a path that stays where it is.
pub(crate) const CURRENT: &str = crate::domain::packages::constants::CURRENT;

/// A part of a path that goes up one directory.
pub(crate) const PARENT: &str = crate::domain::packages::constants::PARENT;

/// What starts an import of a package's own `imports`.
pub(crate) const PACKAGE_IMPORT: char = '#';

/// The directory at the root of the repository.
pub(crate) const ROOT: &str = "";

/// Why an import that names a file of the repository no file is makes the relation read in part, after the specifier.
pub(crate) const UNRESOLVED_CAUSE: &str = "names a file of the repository that no file is";
