//! The names of the `allocation_verdict` concept: how the allowance is set.

use super::types::{AllocatedBlocks, Ceiling, FileCount, TotalBlocks};

/// How many of the first files grow what the worker keeps, and so are not judged.
pub(crate) const WARM_UP: usize = 4;

/// What follows the item in the path of a file.
pub(crate) const ITEM_END: char = '/';

/// How many files, for each doubling of their number, may allocate one block beyond the parser library.
pub(crate) const GROWTHS_PER_DOUBLING: usize = 3;

/// The most blocks one file may allocate beyond the parser library.
pub(crate) const MOST_BEYOND: u64 = 1;

/// The interpreter's ceiling with every feature on (check, coverage): 56 files, 91016 blocks, at most 1627 (CI run 43607885952).
pub(crate) const INTERPRETER_CEILING_ALL_FEATURES: Ceiling = Ceiling {
    most: AllocatedBlocks(1627),
    total: TotalBlocks(91016),
    judged: FileCount(56),
};

/// The interpreter's ceiling with default features (release-tests): 56 files, 90960 blocks, at most 1626 (CI run 43607885952).
pub(crate) const INTERPRETER_CEILING_DEFAULT_FEATURES: Ceiling = Ceiling {
    most: AllocatedBlocks(1626),
    total: TotalBlocks(90960),
    judged: FileCount(56),
};

/// The ceiling of the features this build has, keyed on core's forwarded `diagnostic-row-progress`.
pub(crate) const INTERPRETER_CEILING: Ceiling = if cfg!(feature = "diagnostic-row-progress") {
    INTERPRETER_CEILING_ALL_FEATURES
} else {
    INTERPRETER_CEILING_DEFAULT_FEATURES
};
