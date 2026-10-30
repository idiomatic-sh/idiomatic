//! The types of the `job_scheduling` port: plain data, public fields and derives, with no constructor
//! and no inherent impl.

/// How many jobs to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ScheduledJobCount(pub u32);

/// Which of the jobs is running, counting from zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct JobIndex(pub u32);
