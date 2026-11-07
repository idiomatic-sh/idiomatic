//! core's scheduler workload.
//!
//! The scheduler hex's factory functions back to back: sizing and building the invocation's pool, and running a few jobs on the rayon
//! scheduler over one and on the sequential scheduler, so the faster is chosen by measurement.

use std::sync::atomic::{AtomicU64, Ordering};

use ryo_core::outbound::scheduler::factory::{rayon, sequential};
use ryo_core::outbound::scheduler::interface::SchedulerPort;
use ryo_core::outbound::scheduler::types::{JobCount, JobNumber, WorkerCount};

/// How many workers the pool has.
const ASKED: u32 = 4;

/// How many jobs each run has.
const JOBS: JobCount = JobCount(64);

/// What the hex builds.
#[derive(Clone, Copy, Debug)]
enum Variant {
    /// The pool of workers, sized.
    Pool,
    /// The scheduler over a pool.
    Rayon,
    /// The scheduler on the calling thread.
    Sequential,
}

impl Variant {
    /// Every function of the factory, run in turn.
    const ALL: [Self; 3] = [Self::Pool, Self::Rayon, Self::Sequential];
}

/// One worker: each job sums a thousand numbers and adds the sum, and the job's number, to `total`.
fn job(total: &AtomicU64) -> Box<dyn FnMut(JobNumber) + Send + '_> {
    Box::new(move |number: JobNumber| {
        let sum = divan::black_box((0..1_000_u64).sum::<u64>()) + u64::from(number.0);
        let _before = total.fetch_add(sum, Ordering::Relaxed);
    })
}

/// Runs `JOBS` jobs on `scheduler`, each summing a thousand numbers, and answers their total.
fn run_jobs(scheduler: &dyn SchedulerPort) -> u64 {
    let total = AtomicU64::new(0);
    let count = ASKED;
    let mut each = std::iter::repeat_with(|| job(&total))
        .take(usize::try_from(count).unwrap_or(1))
        .collect::<Vec<_>>();
    let result = scheduler.run(&mut each, JOBS);
    drop(each);
    result.map_or(0, |()| total.load(Ordering::Relaxed))
}

/// Builds what the variant names, and for a scheduler runs the jobs on it.
#[divan::bench(args = Variant::ALL)]
fn build_and_run(variant: Variant) -> u64 {
    let asked = WorkerCount(ASKED);
    match variant {
        Variant::Pool => {
            drop(divan::black_box(rayon(asked)));
            u64::from(asked.0)
        }
        Variant::Rayon => run_jobs(&rayon(asked)),
        Variant::Sequential => run_jobs(&sequential()),
    }
}
