//! The names of the `targets` concept: the marks a target is resolved by.

/// What opens a fragment, which names a place in a file.
pub(super) const FRAGMENT: char = '#';

/// What opens a query.
pub(super) const QUERY: char = '?';

/// What separates the parts of a path.
pub(super) const SEPARATOR: char = '/';

/// What opens a target that names a host, `//cdn.example/a`.
pub(super) const HOST: &str = "//";

/// A part of a path that stays where it is.
pub(super) const HERE: &str = ".";

/// A part of a path that goes up one directory.
pub(super) const UP: &str = "..";

/// What ends the scheme a target opens with.
pub(super) const SCHEME_END: char = ':';

/// The characters a scheme holds besides letters and digits, after its first letter.
pub(super) const SCHEME_CHARACTERS: [char; 3] = ['+', '.', '-'];
