//! The words of the `modules` concept: the names of the files and the words of the paths they declare.

/// The manifest that declares a module.
pub(crate) const MODULE_FILE: &str = "go.mod";

/// The manifest that declares a workspace.
pub(crate) const WORKSPACE_FILE: &str = "go.work";

/// What separates the parts of a path.
pub(crate) const SEPARATOR: char = '/';

/// The directory a relative path starts from.
pub(crate) const HERE: &str = ".";

/// The directory above.
pub(crate) const ABOVE: &str = "..";

/// What the cause of a refusal is joined with.
pub(crate) const CAUSE_SEPARATOR: &str = ": ";

/// The directory of the root of the repository, written as a path.
pub(crate) const ROOT: &str = "";

/// What joins the parts of a path.
pub(crate) const JOINER: &str = "/";

/// The condition of a requirement of Go: it has none.
pub(crate) const NO_CONDITION: &str = "";

/// The part of a path between two separators written together, or before a leading one: it names nothing.
pub(crate) const EMPTY: &str = "";

/// The verb of a `go.mod` directive that requires a module.
pub(crate) const REQUIRE_VERB: &str = "require";

/// The quotes a word of a `go.mod` line can be written in.
pub(crate) const QUOTES: &[char] = &['"', '`'];
