//! The names of the `allocation_verdict` concept: how the allowance is set.

/// How many of the first files grow what the worker keeps, and so are not judged.
pub(crate) const WARM_UP: usize = 4;

/// What follows the item in the path of a file.
pub(crate) const ITEM_END: char = '/';

/// How many files, for each doubling of their number, may allocate one block beyond the parser library.
pub(crate) const GROWTHS_PER_DOUBLING: usize = 3;

/// The most blocks one file may allocate beyond the parser library.
pub(crate) const MOST_BEYOND: u64 = 1;
