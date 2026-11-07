//! The names of the `patterns` concept: the marks a path and a pattern over paths are written with.

/// What separates the parts of a path.
pub(crate) const SEPARATOR: char = '/';

/// A part of a path that stays where it is.
pub(crate) const HERE: &str = ".";

/// A part of a pattern that is any number of parts.
pub(crate) const ANY_PARTS: &str = "**";

/// In a pattern, any run of characters inside a part.
pub(crate) const ANY_RUN: char = '*';

/// In a pattern, any one character.
pub(crate) const ANY_ONE: char = '?';
