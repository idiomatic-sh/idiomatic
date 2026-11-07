//! The names of the `coverage` concept: the text of what a rule says it could not decide.

/// What follows the name of a relation in the message of a rule that could not decide.
pub(crate) const UNKNOWN_IN: &str = " is read only in part in ";

/// What follows the path in that message.
pub(crate) const UNKNOWN_BECAUSE: &str = ": ";

/// What names the repository as a whole where a message names a file.
pub(crate) const REPOSITORY: &str = ".";

/// What a rule says when partial project coverage names no file it can be placed on.
pub(crate) const UNPLACEABLE_COVERAGE: &str =
    "partial project coverage cannot be placed on its declared files";
