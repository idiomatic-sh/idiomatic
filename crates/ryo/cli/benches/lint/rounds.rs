//! The shape of every measurement of the lint bench: how many rounds each cost is taken over, and on how many workers a run is taken.
//! A measure of time or memory depends on the workers a run uses, and the lock holds no machine, so no run asks for as many as the
//! machine has.

/// How many rounds every cost is taken over, but the cold cost of ruff: the median of these is the `value`.
pub(crate) const ROUNDS: usize = 3;

/// How many rounds the cold cost of ruff and its CPU are taken over.
pub(crate) const COLD_ROUNDS: usize = 5;

/// The workers a run over ruff is taken on, in every measurement: the cold cost, the cache rows, the store and the allocation profile.
pub(crate) const WORKERS: usize = 10;
