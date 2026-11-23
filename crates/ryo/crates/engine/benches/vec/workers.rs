//! Partition a closed file inventory. Visiting a file always uses its inventory ordinal.
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::num::NonZeroUsize;

pub(crate) fn count(request: Option<NonZeroUsize>, files: usize) -> usize {
    request
        .or_else(|| std::thread::available_parallelism().ok())
        .map_or(1, NonZeroUsize::get)
        .min(files)
        .max(1)
}

pub(crate) fn divide(
    files: std::ops::Range<usize>,
    workers: usize,
    size: impl Fn(usize) -> std::io::Result<u64>,
) -> std::io::Result<Vec<Vec<usize>>> {
    let start = super::super::thread_cpu();
    let n = workers.min(files.len()).max(1);
    if n == 1 {
        return Ok(vec![files.collect()]);
    }
    let mut jobs = files
        .map(|file| Ok((Reverse(size(file)?), file)))
        .collect::<std::io::Result<Vec<_>>>()?;
    jobs.sort_unstable();
    let mut loads: BinaryHeap<_> = (0..n)
        .map(|worker| Reverse((0u64, 0usize, worker)))
        .collect();
    let mut shards = vec![Vec::new(); n];
    for (Reverse(bytes), file) in jobs {
        let mut next = loads.peek_mut().expect("one queue per worker");
        let Reverse((total, count, worker)) = &mut *next;
        shards[*worker].push(file);
        // Empty files still have a visit. Give them a unit so they spread
        // across byte-balanced shards whose final totals differ by a few bytes.
        *total += bytes.max(1);
        *count += 1;
    }
    for shard in &mut shards {
        shard.sort_unstable();
    }
    println!(
        "worker inventory {} workers CPU {:.6} s",
        n,
        super::super::thread_cpu() - start
    );
    Ok(shards)
}

/// Run one shard on the caller, which can borrow non-Send state. Other shards construct
/// their executor on their own thread and return only their finished, owned records.
pub(crate) fn run<T: Send>(
    shards: &[Vec<usize>],
    primary: impl FnOnce(&[usize]) -> T,
    secondary: impl Fn(&[usize]) -> T + Sync,
) -> Vec<T> {
    #[cfg(target_os = "linux")]
    let mask = (shards.len() > 1)
        .then(|| rustix::thread::sched_getaffinity(None).ok())
        .flatten();
    #[cfg(target_os = "linux")]
    let cpus: Vec<_> = mask
        .iter()
        .flat_map(|mask| (0..rustix::thread::CpuSet::MAX_CPU).filter(|&i| mask.is_set(i)))
        .collect();
    let place = |_index: usize| {
        #[cfg(target_os = "linux")]
        if !cpus.is_empty() {
            let mut one = rustix::thread::CpuSet::new();
            one.set(cpus[_index % cpus.len()]);
            let _ = rustix::thread::sched_setaffinity(None, &one);
        }
    };
    std::thread::scope(|scope| {
        let other: Vec<_> = shards[1..]
            .iter()
            .enumerate()
            .map(|(i, shard)| {
                scope.spawn({
                    let place = &place;
                    let secondary = &secondary;
                    move || {
                        place(i + 1);
                        secondary(shard)
                    }
                })
            })
            .collect();
        #[cfg(target_os = "linux")]
        struct Restore<'a>(Option<&'a rustix::thread::CpuSet>);
        #[cfg(target_os = "linux")]
        impl Drop for Restore<'_> {
            fn drop(&mut self) {
                if let Some(mask) = self.0 {
                    let _ = rustix::thread::sched_setaffinity(None, mask);
                }
            }
        }
        #[cfg(target_os = "linux")]
        let _restore = Restore(mask.as_ref());
        place(0);
        let mut results = vec![primary(&shards[0])];
        for worker in other {
            results.push(
                worker
                    .join()
                    .unwrap_or_else(|panic| std::panic::resume_unwind(panic)),
            );
        }
        results
    })
}
