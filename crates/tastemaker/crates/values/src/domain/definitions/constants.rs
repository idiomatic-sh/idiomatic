//! The names a workspace is loaded by.

/// A workspace's file.
pub const WORKSPACE_FILE: &str = "tastemaker.toml";
/// The lock beside the root file.
pub const LOCK_FILE: &str = "tastemaker.lock";
/// git's ignore file.
pub const GITIGNORE: &str = ".gitignore";
/// What marks a repository's root: a directory, or a file in a linked worktree; every walk skips it.
pub const GIT: &str = ".git"; // ignore: idiomatic.arch/no-language-names - `.git` is the version-control directory every walk skips, not a file extension or a language's file
/// Asymptote's configuration file at a project's top, which `init` leaves unchecked beside the lock: a project that adds Asymptote after
/// `tm init` stays whole.
pub const ASYMPTOTE_FILE: &str = "asymptote.toml";
/// The directory Asymptote keeps its lock in, committed beside the project's files.
pub const ASYMPTOTE_DIRECTORY: &str = ".asymptote";
/// What the store's directory holds besides its files: a note that it is a cache, for backup tools.
pub const TAG: &str = "CACHEDIR.TAG";
/// The note.
pub const TAG_TEXT: &str = "Signature: 8a477f597d28d172789f06886806bc55\n# The store of Tastemaker: rebuilt by any run, safe to delete.\n";
/// What the store's ignore file says: every file of the directory, itself too.
pub const GITIGNORE_TEXT: &str =
    "# The store of Tastemaker: rebuilt by any run, never checked in.\n*\n";

/// The repository's own excludes, which a walk reads too.
pub const GIT_EXCLUDE: &str = ".git/info/exclude";

/// The folded key that ordered a workspace's crates in layers.
pub const LAYERS_KEY: &str = "layers";
/// The folded key that kept crates independent of each other.
pub const INDEPENDENT_KEY: &str = "independent";
/// The folded key that kept crates acyclic.
pub const ACYCLIC_KEY: &str = "acyclic";
/// The manifest's table of the values each instance declares, read by `reporting::mismatch`.
pub const VARIABLES_KEY: &str = "variables";

/// Why a prospective root cannot initialize workspace loading.
pub const MISSING_ROOT: &str = "the prospective manifest has no workspace table";
