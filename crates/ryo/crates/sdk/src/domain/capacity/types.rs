//! The types of the `capacity` concept: the threads every run reads files on, and the most memory a run may use.

use std::num::NonZeroU64;
use std::sync::Arc;

use ryo_core::outbound::scheduler::{interface::SchedulerPort, types::WorkerCount};

/// The threads every run reads files on, built once by the caller and handed to each run, with the scheduler over them: made from a
/// count of `Workers`, or from a pool the caller already runs.
#[derive(Clone, Debug)]
pub struct WorkerPool(Arc<dyn SchedulerPort>, WorkerCount);

impl WorkerPool {
    /// The scheduler built over the pool.
    pub(crate) const fn scheduler(scheduler: Arc<dyn SchedulerPort>, workers: WorkerCount) -> Self {
        Self(scheduler, workers)
    }

    /// How many workers the pool runs jobs on.
    pub(crate) const fn workers(&self) -> WorkerCount {
        self.1
    }

    /// The scheduler, as the engine takes it.
    pub(crate) fn of(self) -> Arc<dyn SchedulerPort> {
        self.0
    }
}

/// The most memory a run may use: the files it reads must fit in it together, and the run stops when the process (the program that
/// called it included) is found holding more while it reads them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MemoryBudget {
    /// What the machine has.
    Machine,
    /// This many bytes.
    Bytes(NonZeroU64),
}

// The conversion into the engine's budget is the crate's own: a public `From` impl would put the engine's types on the SDK's surface.
impl MemoryBudget {
    /// The most memory a run may use, as the memory factory takes it.
    pub(crate) const fn into_engine(self) -> ryo_values::domain::runs::types::Budget {
        use ryo_values::domain::runs::types::{Budget, MemoryBytes};
        match self {
            Self::Bytes(bytes) => Budget::Bytes(MemoryBytes(bytes.get())),
            Self::Machine => Budget::Machine,
        }
    }
}
