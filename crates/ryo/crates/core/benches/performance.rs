//! core's workloads.

#[path = "performance/corpus/bench.rs"]
mod corpus;

#[path = "performance/facts/bench.rs"]
mod facts;

#[path = "performance/lint_store/bench.rs"]
mod lint_store;

#[path = "performance/scheduler/bench.rs"]
mod scheduler;

fn main() {
    divan::Divan::default()
        .max_time(std::time::Duration::from_secs(10))
        .config_with_args()
        .main();
}
