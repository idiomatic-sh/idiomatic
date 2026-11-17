//! Environment names a measured run treats specially.
/// The variable a contestant's thread pool takes its thread count from: a timed run pins it, a
/// correctness comparison leaves it to the contestant's default.
pub const THREAD_PIN: &str = "RAYON_NUM_THREADS";
/// The workers every timed measure of a bench runs with: the bench tool's run sets `THREAD_PIN` to it, and the processors
/// to `taskset -c 0-3`, once, for the whole process tree.
pub const MEASURED_WORKERS: usize = 4;
/// The variable cargo takes its job count from: clippy's reference runs on `MEASURED_WORKERS` jobs.
pub const JOBS_PIN: &str = "CARGO_BUILD_JOBS";
/// The flag whose next argument is a contestant's thread count: oxlint's reference runs on `MEASURED_WORKERS` threads.
pub const THREADS_FLAG: &str = "--threads";
