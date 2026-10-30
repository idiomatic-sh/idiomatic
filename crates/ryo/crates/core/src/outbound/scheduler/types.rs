//! The types of the `scheduler` port: plain data, public fields and derives, with no constructor
//! and no inherent impl.

/// How many jobs to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobCount(pub u32);

/// Which of the jobs a worker is running, counting from zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobNumber(pub u32);

/// How many workers a scheduler runs jobs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkerCount(pub u32);
