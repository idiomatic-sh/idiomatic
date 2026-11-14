//! The names of the `decoded` concept: the room one binary64 spelling takes, and what that spelling is made of.

/// `Display`'s longest finite binary64 spelling, with its `.0` suffix, fits without a heap allocation.
pub const FLOAT_TEXT_BYTES: usize = 350;
/// The text `numeric Display only writes ASCII`.
pub(crate) const ASCII_ONLY: &str = "numeric Display only writes ASCII";
/// The suffix that keeps a whole-valued binary64 spelling a float.
pub(crate) const FLOAT_SUFFIX: &[u8] = b".0";
