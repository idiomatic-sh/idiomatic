//! Run independent, numbered jobs through the scheduler port, from the pool's owner thread (inside `on_owner`). Collect in
//! admission order and count CPU on other threads separately from the session clock.
use ryo_core::outbound::scheduler::{interface::SchedulerPort, types::{JobCount, JobNumber}};
use std::sync::Mutex;

/// Runs `work` on each of `jobs`, once each, on `workers` of the scheduler's threads. One port call for the whole set.
///
/// # Panics
///
/// When the scheduler refuses the set: the edge that built it has run its owner job on it already, so it is live.
pub fn run<I, T, F>(scheduler: &dyn SchedulerPort, workers: usize, jobs: Vec<I>, work: F) -> (Vec<T>, f64)
where
    I: Send,
    T: Send,
    F: Fn(I) -> T + Send + Sync,
{
    let owner = std::thread::current().id();
    let count = jobs.len();
    let jobs: Vec<Mutex<Option<I>>> = jobs.into_iter().map(|job| Mutex::new(Some(job))).collect();
    let slots: Vec<Mutex<Option<(T, f64)>>> = (0..count).map(|_| Mutex::new(None)).collect();
    let mut pool: Vec<Box<dyn FnMut(JobNumber) + Send + '_>> = (0..workers.clamp(1, count.max(1)))
        .map(|_| {
            let (work, jobs, slots) = (&work, &jobs, &slots);
            Box::new(move |JobNumber(job): JobNumber| {
                let start = (std::thread::current().id() != owner).then(super::thread_cpu);
                let item = jobs[job as usize].lock().unwrap_or_else(std::sync::PoisonError::into_inner).take().expect("a job runs once");
                let result = work(item);
                let cpu = start.map_or(0., |start| super::thread_cpu() - start);
                *slots[job as usize].lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some((result, cpu));
            }) as Box<dyn FnMut(JobNumber) + Send + '_>
        })
        .collect();
    scheduler
        .run(&mut pool, JobCount(u32::try_from(count).expect("job count")))
        .expect("the scheduler runs the jobs");
    drop(pool);
    let mut child_cpu = 0.;
    let values = slots
        .into_iter()
        .map(|slot| {
            let (value, cpu) = slot.into_inner().unwrap_or_else(std::sync::PoisonError::into_inner).expect("every job ran");
            child_cpu += cpu;
            value
        })
        .collect();
    (values, child_cpu)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ryo_core::outbound::scheduler::errors::SchedulerError;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A port that runs the jobs in the caller and counts how often it is called.
    #[derive(Debug, Default)]
    struct Counting(AtomicUsize);

    impl SchedulerPort for Counting {
        fn run(&self, workers: &mut [Box<dyn FnMut(JobNumber) + Send + '_>], count: JobCount) -> Result<(), SchedulerError> {
            self.0.fetch_add(1, Ordering::Relaxed);
            for job in 0..count.0 {
                workers[0](JobNumber(job));
            }
            Ok(())
        }
    }

    #[test]
    fn a_set_of_jobs_is_one_port_call_and_answers_in_admission_order() {
        let port = Counting::default();
        let rows = AtomicUsize::new(0);
        let (values, _) = run(&port, 4, (0..50).collect(), |job: usize| {
            rows.fetch_add(1000, Ordering::Relaxed);
            job * 2
        });
        assert_eq!(values, (0..50).map(|job| job * 2).collect::<Vec<_>>());
        assert_eq!(port.0.load(Ordering::Relaxed), 1, "one port call for the set, never one per job or per row");
        assert_eq!(rows.load(Ordering::Relaxed), 50_000);
    }

    #[test]
    fn an_empty_set_is_one_call_with_no_jobs() {
        let port = Counting::default();
        let (values, cpu) = run(&port, 3, Vec::<usize>::new(), |job| job);
        assert!(values.is_empty());
        assert!(cpu.abs() < f64::EPSILON);
    }
}
