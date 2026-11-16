//! What the meter reads.

/// The CPU time (user and system) this process has used so far.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CpuTime(pub std::time::Duration);
