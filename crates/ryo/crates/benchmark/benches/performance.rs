//! benchmark's workloads.
#[path = "performance/engine/bench.rs"]
mod engine;
#[path = "performance/evidence/bench.rs"]
mod evidence;
#[path = "performance/execution/bench.rs"]
mod execution;
#[path = "performance/linter/bench.rs"]
mod linter;
fn main() {
    divan::main();
}
