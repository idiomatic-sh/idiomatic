//! The words of the `visibilities` concept: the accessibilities a class member is written with.

/// The accessibility modifiers of TypeScript (`public`, `private`, `protected`).
pub(crate) const ACCESSIBILITIES: [&str; 3] = ["public", "private", "protected"];

/// What goes between two keywords of a visibility.
pub(crate) const SPACE: &str = " ";
