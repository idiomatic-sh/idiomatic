//! The fake of the `job_scheduling` port: what the provider's tests stand in for it with.

use super::super::interface::JobSchedulingPort;
use crate::outbound::job_scheduling::types::{JobIndex, ScheduledJobCount};

/// What the provider's tests stand in for `job_scheduling` with: it runs the jobs in order, one after the other, on the thread that calls it.
#[derive(Debug)]
pub(crate) struct JobSchedulingFake;

impl JobSchedulingPort for JobSchedulingFake {
    fn run(&self, count: ScheduledJobCount, job: &(dyn Fn(JobIndex) + Sync)) {
        for index in 0..count.0 {
            job(JobIndex(index));
        }
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
