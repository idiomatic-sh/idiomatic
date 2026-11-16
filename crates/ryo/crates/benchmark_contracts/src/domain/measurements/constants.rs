//! Environment names a measured run treats specially.
/// The variable a contestant's thread pool takes its thread count from: a timed run pins it, a
/// correctness comparison leaves it to the contestant's default.
pub const THREAD_PIN: &str = "RAYON_NUM_THREADS";
/// The workers every timed measure of a bench runs with: the bench tool's run sets `THREAD_PIN` to it, and the processors
/// to `taskset -c 0-3`, once, for the whole process tree.
pub const MEASURED_WORKERS: usize = 4;
