//! How many rounds each cost of the lint bench is taken over.

/// How many rounds every cost is taken over, but the cold cost of ruff: the median of these is the `value`.
pub(crate) const ROUNDS: usize = 3;

/// How many rounds the cold cost of ruff and its CPU are taken over.
pub(crate) const COLD_ROUNDS: usize = 5;
