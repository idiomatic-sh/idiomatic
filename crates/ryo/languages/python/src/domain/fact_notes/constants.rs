//! The names of the `fact_notes` concept: how the imports a file writes are noted.

/// What separates the parts of a dotted path.
pub(crate) const DOT: char = '.';

/// The calls that load a module by the name their first argument gives, as the call is written: `__import__("a")` and
/// `importlib.import_module("a.b")`.
pub(crate) const LOADERS: [&str; 2] = ["__import__", "importlib.import_module"];
