//! The names of the `numeric_literals` concept.
/// `Display`'s longest finite binary64 spelling, with its `.0` suffix, fits without a heap allocation.
pub(crate) const FLOAT_TEXT_BYTES: usize = 350;
