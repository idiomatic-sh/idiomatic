//! The `runs` service: free functions named by verb over what a run is asked, with no IO and no ports.

use crate::domain::runs::types::{MachineWorkers, StartedWorkers, WorkersAsked};

/// How many workers the run starts: what the caller asked for, at most what the machine has; the machine's when it asked for none.
#[must_use]
pub fn workers_to_start(asked: WorkersAsked, machine: MachineWorkers) -> StartedWorkers {
    match u32::try_from(asked.0) {
        Ok(0) | Err(_) => StartedWorkers(machine.0),
        Ok(count) => StartedWorkers(count.min(machine.0)),
    }
}
