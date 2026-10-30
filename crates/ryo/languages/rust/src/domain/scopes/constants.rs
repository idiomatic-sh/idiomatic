//! The names of the `scopes` concept: where the file itself is.

use ryo::languages;

/// Where the file itself is, as an owner: the root of its tree.
pub(crate) const FILE_OWNER: languages::Position = languages::Position(0);
