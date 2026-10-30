//! The types of the `runs` concept: what walking the text of a run keeps track of. Plain data and derives, with no constructor and
//! no inherent impl.

/// Whether a space is owed before the next word of a run.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Gap {
    /// The next word follows the last directly.
    Closed,
    /// The next word follows a space.
    Open,
}
