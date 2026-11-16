//! The names of the `allocation_verdict` concept: how the allowance is set.

/// How many of the first files grow what the worker keeps, and so are not judged.
pub(crate) const WARM_UP: usize = 4;

/// What follows the item in the path of a file.
pub(crate) const ITEM_END: char = '/';
