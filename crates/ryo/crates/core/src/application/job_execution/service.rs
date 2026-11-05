//! Execute one existing ready batch through the injected common scheduler.
use super::types::TaskIndex;
use crate::outbound::scheduler::{
    errors::SchedulerError,
    interface::SchedulerPort,
    types::{JobCount, JobNumber, WorkerCount},
};
use std::sync::{
    Mutex, PoisonError,
    atomic::{AtomicUsize, Ordering},
};

pub(crate) fn drain<I: Sync, T: Send>(
    scheduler: &dyn SchedulerPort,
    workers: WorkerCount,
    tasks: &[I],
    work: impl Fn(TaskIndex, &I) -> T + Sync,
) -> Result<Vec<T>, SchedulerError> {
    let total = JobCount(u32::try_from(tasks.len()).unwrap_or(u32::MAX));
    let slots: Vec<Mutex<Option<T>>> = std::iter::repeat_with(|| Mutex::new(None))
        .take(tasks.len())
        .collect();
    let next = AtomicUsize::new(0);
    let mut started: Vec<Box<dyn FnMut(JobNumber) + Send + '_>> = Vec::new();
    for _ in 0..workers.0.min(total.0) {
        let (slots, next, work) = (&slots, &next, &work);
        started.push(Box::new(move |_job: JobNumber| {
            loop {
                let task = next.fetch_add(1, Ordering::Relaxed);
                let (Some(slot), Some(item)) = (slots.get(task), tasks.get(task)) else {
                    break;
                };
                *slot.lock().unwrap_or_else(PoisonError::into_inner) =
                    Some(work(TaskIndex(task), item));
            }
        }));
    }
    let count = JobCount(u32::try_from(started.len()).unwrap_or(u32::MAX));
    scheduler.run(&mut started, count)?;
    drop(started);
    Ok(slots
        .into_iter()
        .map(|slot| {
            slot.into_inner()
                .unwrap_or_else(PoisonError::into_inner)
                .expect("completed scheduler fills every task slot")
        })
        .collect())
}

/// Open a fixed set of independent jobs and return their answers in job order.
pub(crate) fn run_indexed<T: Send>(
    scheduler: &dyn SchedulerPort,
    workers: WorkerCount,
    count: u32,
    work: impl Fn(JobNumber) -> T + Sync,
) -> Result<Vec<T>, SchedulerError> {
    let jobs: Vec<JobNumber> = (0..count).map(JobNumber).collect();
    drain(scheduler, WorkerCount(workers.0.max(1)), &jobs, |_, job| {
        work(*job)
    })
}
