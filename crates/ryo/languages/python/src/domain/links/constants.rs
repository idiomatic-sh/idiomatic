//! The names of the `links` concept: the files a module is, and the text a finding about the links shows.

/// The ending of a module's file.
pub(crate) const MODULE: &str = ".py";

/// The ending of a module's stub.
pub(crate) const STUB: &str = ".pyi";

/// The file a regular package is.
pub(crate) const PACKAGE: &str = "__init__";

/// The directory a project in the src layout keeps its import packages in.
pub(crate) const SOURCE_LAYOUT: &str = "src";

/// What separates the parts of a path.
pub(crate) const SEPARATOR: char = '/';

/// What separates the parts of a dotted path.
pub(crate) const DOT: char = '.';

/// Why an import that names a module of the repository no file is makes the relation read in part, after the import.
pub(crate) const UNRESOLVED_CAUSE: &str = "names a module of the repository that no file is";

/// The directory at the root of the repository, and the path of a module no part names.
pub(crate) const ROOT: &str = crate::domain::packages::constants::ROOT;
