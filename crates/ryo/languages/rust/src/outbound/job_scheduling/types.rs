//! The jobs the `job_scheduling` port runs.

/// Jobs to run at once: each owns what it writes and borrows only what it reads.
pub(crate) struct Jobs<'a>(pub Vec<Box<dyn FnOnce() + Send + 'a>>);
