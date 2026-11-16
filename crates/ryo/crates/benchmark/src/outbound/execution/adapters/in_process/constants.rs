//! The text this part uses.
/// The exit status of a worker request that completed.
pub(super) const COMPLETED: i32 = 0;
/// The exit status of a worker request that failed.
pub(super) const FAILED: i32 = 1;
/// The refusal of an invocation that is no worker request: only the worker runs in this process.
pub(super) const NOT_A_WORKER_REQUEST: &str =
    "the in-process execution runs only a worker request; every other invocation runs as a child";
