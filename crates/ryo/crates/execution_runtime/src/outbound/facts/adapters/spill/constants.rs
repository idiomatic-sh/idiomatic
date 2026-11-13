//! The constants of the spill adapter.

/// The start of the name of a spill file, which is removed from the directory as soon as it is made.
pub(crate) const FILE: &str = ".ryo_spill";

/// How many bytes wait in memory before they are written.
pub(crate) const FLUSH_AT: usize = 1 << 20;

/// What a refusal says when the handle given to the finish is not one this adapter opened, or a relation has no columns.
pub(crate) const MISSING: &str = "no such handle or relation";
