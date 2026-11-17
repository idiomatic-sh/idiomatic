//! Runs every task on a scoped thread of its own and joins them all.
use crate::outbound::scheduler::interface::SchedulerPort;

/// The scoped-thread scheduler.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SchedulerThreadsAdapter;

impl SchedulerPort for SchedulerThreadsAdapter {
    fn run_all(&self, tasks: Vec<Box<dyn FnOnce() + Send + '_>>) {
        std::thread::scope(|scope| {
            for task in tasks {
                let _worker = scope.spawn(task);
            }
        });
    }
}
