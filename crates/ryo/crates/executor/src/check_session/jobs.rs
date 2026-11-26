//! Run independent, numbered jobs in the caller's active Rayon pool. Collect in
//! admission order and count CPU on other threads separately from the session clock.
use rayon::prelude::*;

pub fn run<I, T, F>(jobs: I, work: F) -> (Vec<T>, f64)
where
    I: IntoParallelIterator<Iter: IndexedParallelIterator>,
    T: Send,
    F: Fn(I::Item) -> T + Send + Sync,
{
    let owner = std::thread::current().id();
    let results: Vec<_> = jobs.into_par_iter().map(|job| {
        let start = (std::thread::current().id() != owner).then(super::thread_cpu);
        let result = work(job);
        (result, start.map_or(0., |start| super::thread_cpu() - start))
    }).collect();
    let mut child_cpu = 0.;
    let values = results.into_iter().map(|(value, cpu)| {
        child_cpu += cpu;
        value
    }).collect();
    (values, child_cpu)
}
