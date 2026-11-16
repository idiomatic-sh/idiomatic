//! The shape of every measurement of the lint bench: how many rounds each cost is taken over, and on how many workers a run is taken.
//! A measure of time or memory depends on the workers a run uses, and the lock holds no machine, so no run asks for as many as the
//! machine has.

/// How many rounds every cost is taken over, but the cold cost of ruff: the median of these is the `value`.
pub(crate) const ROUNDS: usize = 3;

/// How many rounds the cold cost of ruff and its CPU are taken over.
pub(crate) const COLD_ROUNDS: usize = 5;

/// The workers the cold cost of ruff is taken on.
pub(crate) const WORKERS: usize = 10;

/// The workers every memory measurement is taken on: the cache rows, the store and the allocation profile. The engine starts no more
/// workers than the machine has, so a count above what the smallest machine has is a count that machine does not run: four is what a
/// runner has, and the memory runs are held to four processors (`taskset -c 0-3`), which [`require_pinned`] checks.
pub(crate) const MEMORY_WORKERS: usize = 4;

/// Refuses a memory run that is not held to exactly [`MEMORY_WORKERS`] processors: on fewer (or on a machine with fewer than four, where
/// `taskset -c 0-3` fails first) the workers differ from the lock's, and on more it is not pinned.
///
/// # Panics
///
/// When the process may use any other number of processors.
pub(crate) fn require_pinned() {
    let processors = std::thread::available_parallelism().map_or(0, std::num::NonZero::get);
    assert_eq!(
        processors, MEMORY_WORKERS,
        "a memory run is held to {MEMORY_WORKERS} processors, as `taskset -c 0-3 cargo bench ...` does; this process may use {processors}"
    );
}
