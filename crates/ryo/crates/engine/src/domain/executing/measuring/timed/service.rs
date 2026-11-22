//! The services of the `timed` part: the language's work kept on a worker's clock as it is timed.

use crate::domain::counting::types::{FillClock, ProviderFill, ProviderWork};
use cpu_time::ThreadTime;
use ryo_relations::domain::relations::types::Count;
use std::sync::atomic::Ordering;
use std::time::Duration;

/// The thread's own time; nought where the system has no such clock.
pub(crate) fn thread_time() -> Duration {
    ThreadTime::try_now().map_or(Duration::ZERO, |now| now.as_duration())
}

/// The language did `work`, writing `rows` rows, since `begun`: kept on `clock` for the file, and owed back by the unit
/// running, whose own time it is not.
pub(crate) fn provider_spent(clock: &FillClock, work: ProviderWork, rows: Count, begun: Duration) {
    let took = thread_time().saturating_sub(begun);
    let nanos = u64::try_from(took.as_nanos()).unwrap_or(u64::MAX);
    let _before = clock.unpaid.fetch_add(nanos, Ordering::Relaxed);
    if let Ok(mut done) = clock.done.lock() {
        done.push(ProviderFill {
            work,
            rows,
            nanos: Count(usize::try_from(nanos).unwrap_or(usize::MAX)),
        });
    }
}
