//! Environment names a measured run treats specially.
/// The variable a contestant's thread pool takes its thread count from: a timed run pins it, a
/// correctness comparison leaves it to the contestant's default.
pub const THREAD_PIN: &str = "RAYON_NUM_THREADS";
