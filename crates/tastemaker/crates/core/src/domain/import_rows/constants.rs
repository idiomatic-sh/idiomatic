//! The names the built-in rules over the import relations read.

/// The relation of the paths that name another file.
pub const EDGES_RELATION: &str = "source-dependency";
/// The relation of the names a `use` brings in.
pub const NAMES_RELATION: &str = "static-import";
/// The relation of the packages a manifest depends on.
pub const PACKAGES_RELATION: &str = "package-dependency";
/// The relation of the package each file belongs to.
pub const OWNERS_RELATION: &str = "file-package";
/// The column a row of the owners is the finding of, and the file it says the package of.
pub const PATH_COLUMN: &str = "path";
/// The key of a row of the owners.
pub const OWNER_KEY: &str = "\"path\"";
/// The column of the manifest a file's package is.
pub const PACKAGE: &str = "package";
/// The column a row of the edges and the packages is the finding of.
pub const SOURCE_COLUMN: &str = "source";
/// The column a row of the names is the finding of.
pub const FILE_COLUMN: &str = "file";
/// The key of a row of the edges.
pub const EDGE_KEY: &str = "\"target\", \"span\"";
/// The key of a row of the names.
pub const NAME_KEY: &str = "\"occurrence\"";
/// The key of a row of the packages.
pub const PACKAGE_KEY: &str = "\"target\", \"binding\", \"kind\"";
/// The column of what a row reaches.
pub const TARGET: &str = "target";
/// The column of where a row is written.
pub const SPAN: &str = "span";
/// The column of the name a package is bound by.
pub const BINDING: &str = "binding";
/// The column of an edge's path as its file writes it at the edge's place, its whitespace left out.
pub const WRITTEN: &str = "written";
