//! The names of the `nodes` concept: the standard the node kinds are named by.

/// The standard of the node kinds, and its version: the ASDL of `CPython`'s `ast` module, pinned in `tests/fixtures/standard/Python.asdl`.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const STANDARD: &str = "CPython 3.15.0rc2 ast";
