//! The fake of the `scheduler` port: what the core's tests stand in for it with.

use std::sync::Mutex;

use super::super::errors::SchedulerError;
use super::super::interface::SchedulerPort;
use crate::outbound::scheduler::types::{JobCount, JobNumber};

/// What the core's tests stand in for `scheduler` with: it runs the jobs in order on one worker, refuses when none was given, or refuses to start.
#[derive(Clone, Copy, Debug, Default)]
pub struct SchedulerFake {
    /// Whether it refuses to start.
    pub fails: bool,
}

impl SchedulerPort for SchedulerFake {
    fn run(
        &self,
        workers: &mut [Box<dyn FnMut(JobNumber) + Send + '_>],
        count: JobCount,
    ) -> Result<(), SchedulerError> {
        let refusal = || SchedulerError::Unavailable {
            reason: "the fake was told to fail".to_owned(),
        };
        if self.fails {
            return Err(refusal());
        }
        let worker = workers.first_mut().ok_or(SchedulerError::NoWorkers)?;
        for index in 0..count.0 {
            worker(JobNumber(index));
        }
        Ok(())
    }
}

/// What a test stands in for `scheduler` with to measure the per-file path: it reads the files one after another on this thread and
/// counts the blocks each file's job allocated, and nothing around it.
#[derive(Debug, Default)]
pub struct CountingSchedulerFake {
    /// The blocks each job of the last run allocated, in the order the jobs ran.
    pub blocks: Mutex<Vec<u64>>,
}

impl SchedulerPort for CountingSchedulerFake {
    fn run(
        &self,
        workers: &mut [Box<dyn FnMut(JobNumber) + Send + '_>],
        count: JobCount,
    ) -> Result<(), SchedulerError> {
        let mut each = Vec::with_capacity(usize::try_from(count.0).unwrap_or(0));
        if let Some(job) = workers.first_mut() {
            for index in 0..count.0 {
                let before = dhat::HeapStats::get().total_blocks;
                job(JobNumber(index));
                each.push(dhat::HeapStats::get().total_blocks - before);
            }
        }
        if let Ok(mut blocks) = self.blocks.lock() {
            *blocks = each;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
