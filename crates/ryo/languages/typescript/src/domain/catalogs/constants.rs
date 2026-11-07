//! The words of dependency specs and catalogs.

/// What a spec starts with when it points at a catalog.
pub(crate) const CATALOG_PREFIX: &str = "catalog:";
/// The catalog name that means the default catalog, besides the empty name.
pub(crate) const DEFAULT: &str = "default";
/// What a spec starts with when it names another package.
pub(crate) const ALIAS_PREFIX: &str = "npm:";
/// What a spec starts with when it names a directory.
pub(crate) const FILE_PREFIX: &str = "file:";
/// The manifest a `file:` spec points at.
pub(crate) const FILE_MANIFEST: &str = "package.json";
/// What a spec starts with when it links a directory.
pub(crate) const LINK_PREFIX: &str = "link:";
/// What a spec starts with when it names a member of the workspace.
pub(crate) const WORKSPACE_PREFIX: &str = "workspace:";
/// What a spec starts with when it names a git repository: a git URL or a host's shorthand.
pub(crate) const GIT_PREFIXES: [&str; 6] =
    ["git+", "git:", "git://", "github:", "gitlab:", "bitbucket:"];
/// What a spec starts with when it names an archive by its URL.
pub(crate) const URL_PREFIXES: [&str; 2] = ["http://", "https://"];
/// What ends the URL of a git repository.
pub(crate) const GIT_SUFFIX: &str = ".git";
/// What the `#` of a git spec starts with when it names a version range, not a revision.
pub(crate) const SEMVER_PREFIX: &str = "semver:";
/// What a Yarn resolution's key starts with when it replaces a package wherever it is in the tree.
pub(crate) const ANYWHERE: &str = "**/";
/// What a spec starts with when it is a path to a directory written without `file:`.
pub(crate) const PATH_STARTS: [&str; 2] = ["./", "../"];
