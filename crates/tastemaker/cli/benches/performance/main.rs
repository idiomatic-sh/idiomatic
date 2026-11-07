//! Tastemaker's perf entry point: every command's workload and the core's own `check`, each taken for its rounds. It
//! prints one Bencher Metric Format document on stdout (benchmark = the workload, measure = `cpu-ms`, the process's own
//! CPU time: a busy host moves it far less than wall time); Asymptote's `perf` group judges it against the ratchet, the
//! bench asserts no budget. A setup or a run that fails panics, which fails the bench.
//!
//! Run it with `cargo bench --locked -p idiomatic-sh-tastemaker --bench commands`.

mod fixtures;
mod measure;
mod use_case;
mod workspace;

fn main() {
    let mut measured = workspace::measured();
    measured.extend(use_case::measured());
    measure::print(&measured);
}
