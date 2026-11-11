//! The types of the `job_scheduling` port: plain data, public fields and derives, with no constructor
//! and no inherent impl.

/// One job: it runs once, on any worker, and writes only what it owns.
pub(crate) type Job<'a> = Box<dyn FnOnce() + Send + 'a>;
