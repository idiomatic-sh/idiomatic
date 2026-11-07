//! The names of the `workspaces` concept: what a workspace's member patterns are cut with.

/// The characters that make a member a pattern.
pub(crate) const PATTERN: [char; 4] = ['*', '?', '[', '{'];

/// What a member may start with that stands for the directory it is written in.
pub(crate) const HERE: &str = crate::domain::paths::constants::HERE;

/// What a pattern is cut at.
pub(crate) const SEPARATOR: char = '/';

/// What names the directory above in a pattern.
pub(crate) const UP: &str = crate::domain::paths::constants::UP;

/// What stands for a manifest that has not been told yet.
pub(crate) const PENDING: &str = "not read yet";
