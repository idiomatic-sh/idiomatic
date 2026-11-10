//! Tokens whose syntax keeps a glob in the interpreter.

use super::types::GlobText;

/// A directory wildcard has path semantics beyond an ordinary text star.
pub(crate) const DIRECTORY_STARS: GlobText<'static> = GlobText("**");
