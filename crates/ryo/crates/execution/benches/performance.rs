//! The execution crate's workloads.

#[path = "performance/derived/bench.rs"]
mod derived;

#[path = "performance/facts/bench.rs"]
mod facts;

fn main() {
    divan::Divan::default()
        .max_time(std::time::Duration::from_secs(10))
        .config_with_args()
        .main();
}
