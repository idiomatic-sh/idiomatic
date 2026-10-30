//! The names of the `gomod` adapter: the words of the line grammar.

/// What a comment starts with.
pub(super) const COMMENT: &str = "//";

/// What a comment says of a requirement that the code of the module does not need itself.
pub(super) const INDIRECT: &str = "indirect";

/// What opens a block.
pub(super) const OPEN: &str = "(";

/// What closes a block.
pub(super) const CLOSE: &str = ")";

/// What a replacement is written with between the module and what replaces it.
pub(super) const ARROW: &str = "=>";

/// The directive that names the module.
pub(super) const MODULE: &str = "module";

/// The directive that requires a module.
pub(super) const REQUIRE: &str = "require";

/// The directive that replaces a module.
pub(super) const REPLACE: &str = "replace";

/// The directive that uses a directory in a workspace.
pub(super) const USE: &str = "use";

/// The quotes a word may be written between.
pub(super) const QUOTES: [char; 2] = ['"', '`'];

/// What ends a word that is not quoted, besides white space.
pub(super) const DELIMITERS: &str = "()";
