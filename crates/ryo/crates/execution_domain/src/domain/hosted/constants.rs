//! The names of the `hosted` concept: the text that separates the cells of a line of a configuration.

/// What separates the cells of a line of a configuration.
#[must_use]
pub(crate) const fn tab() -> &'static str {
    "\t"
}
