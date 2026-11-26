//! Shared clocks for benchmark evidence.

pub(super) fn thread_cpu() -> f64 {
    thread_local! { static START: cpu_time::ThreadTime = cpu_time::ThreadTime::now(); }
    START.with(|start| start.elapsed().as_secs_f64())
}
