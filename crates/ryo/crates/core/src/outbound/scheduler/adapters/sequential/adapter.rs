//! The scheduler port, implemented in order on the calling thread, on one worker.

use crate::outbound::scheduler::errors::SchedulerError;
use crate::outbound::scheduler::interface::SchedulerPort;
use crate::outbound::scheduler::types::{JobCount, JobNumber};


