//! The operators crate's workloads.

#[path = "performance/kernels/bench.rs"]
mod kernels;

fn main() {
    divan::Divan::default()
        .max_time(std::time::Duration::from_secs(10))
        .config_with_args()
        .main();
}
