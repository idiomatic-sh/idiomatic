//! Runs the tasks one after another on the calling thread, in the order given.
use crate::outbound::scheduler::interface::SchedulerPort;

/// Deterministic scheduler substitute.
#[derive(Clone, Copy, Debug, Default)]
pub struct SchedulerFake;

impl SchedulerPort for SchedulerFake {
    fn run_all(&self, tasks: Vec<Box<dyn FnOnce() + Send + '_>>) {
        for task in tasks {
            task();
        }
    }
}
#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
