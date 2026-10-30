//! The constants of the `files` concept: the text its service looks for.

/// The separator the engine writes between directories of a path.
#[must_use]
pub fn separator() -> &'static str {
    "/"
}
