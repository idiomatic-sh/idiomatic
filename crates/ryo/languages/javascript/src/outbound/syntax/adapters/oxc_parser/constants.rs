//! The text the `oxc_parser` adapter reads with.

/// What the adapter says of bytes that are not UTF-8.
pub(super) const NOT_UTF8: &str =
    "invalid UTF-8: the language reads a file as UTF-8 text, and the parser reads it no further";

/// What the adapter says of a JSX spread child, `{...children}`, which the `ESTree` JSX extension has no place for.
pub(super) const SPREAD_CHILD: &str = "a JSX spread child is not JSX: the ESTree JSX extension (acorn-jsx) refuses `{...expression}` as a child";
