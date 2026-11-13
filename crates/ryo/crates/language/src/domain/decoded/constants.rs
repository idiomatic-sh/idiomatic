//! The names of the `decoded` concept: the room one binary64 spelling takes, and what that spelling is made of.

/// `Display`'s longest finite binary64 spelling, with its `.0` suffix, fits without a heap allocation.
pub const FLOAT_TEXT_BYTES: usize = 350;
