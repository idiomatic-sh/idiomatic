//! The fake of the `job_scheduling` port: what the provider's tests stand in for it with.

use super::super::interface::JobSchedulingPort;

/// What the provider's tests stand in for `job_scheduling` with: it runs the jobs in order, one after the other, on the thread that calls it.
#[derive(Debug)]
pub(crate) struct JobSchedulingFake;

impl JobSchedulingPort for JobSchedulingFake {
    fn run(&self, jobs: &mut [Box<dyn FnMut() + Send + '_>]) {
        for job in jobs.iter_mut() {
            job();
        }
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
