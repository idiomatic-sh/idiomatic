//! The scheduler hex's factory functions back to back: building the invocation's pool, and running a job on the rayon
//! scheduler over one.

use tastemaker_core::outbound::scheduler::factory::{pool, rayon};
use tastemaker_core::outbound::scheduler::interface::SchedulerPort as _;
use tastemaker_core::outbound::scheduler::types::WorkerCount;

/// How many workers the pool has.
const WORKERS: usize = 4;

/// What the hex builds.
#[derive(Clone, Copy, Debug)]
enum Variant {
    /// The pool of workers.
    Pool,
    /// The scheduler over a pool.
    Rayon,
}

impl Variant {
    /// Every function of the factory, run in turn.
    const ALL: [Self; 2] = [Self::Pool, Self::Rayon];
}

fn main() {
    divan::main();
}

/// Builds what the variant names, and for the scheduler runs one job on it.
#[divan::bench(args = Variant::ALL)]
fn build_and_run(variant: Variant) -> u64 {
    let workers = WorkerCount(WORKERS);
    match variant {
        Variant::Pool => {
            drop(divan::black_box(pool(workers)));
            0
        }
        Variant::Rayon => {
            let scheduler = rayon(pool(workers));
            let mut total = 0_u64;
            let result = scheduler.run(&mut || total = divan::black_box((0..1_000_u64).sum()));
            divan::black_box(result.map_or(0, |()| total))
        }
    }
}
