//! The sizes of the rayon pool.

/// The stack each worker of the pool runs on: the work of every scenario of a validation runs on these workers, a lint
/// call of ryo inside each, and a worker that waits for work it handed out takes other work on top of its stack. Only
/// the pages a worker touches are memory.
pub(super) const STACK_BYTES: usize = 64 * 1024 * 1024;
