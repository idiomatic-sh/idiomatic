//! Tastemaker's perf entry point: every command's workload and the core's own `check`, each taken for its rounds. It
//! prints one Bencher Metric Format document on stdout (benchmark = the workload, measure = `cpu-ms`, the process's own
//! CPU time: a busy host moves it far less than wall time); Asymptote's `perf` group judges it against the ratchet, the
//! bench asserts no budget. A setup or a run that fails panics, which fails the bench.
//!
//! Its `workspace/check-ryo-cold` is the peak memory (`peak-kib`) of the command line's cold `tm check` over `crates/ryo`, on Linux,
//! where `/usr/bin/time` reports it; `-- --memory` runs that alone, held to four processors (`taskset -c 0-3`), and no flag runs every other measure.
//!
//! Run it with `cargo bench --locked -p idiomatic-sh-tastemaker --bench commands`.

mod fixtures;
mod measure;
mod use_case;
mod workspace;

fn main() {
    // `-- --memory` takes the memory of the command line alone (the part a pull request can afford).
    let measured = if std::env::args().any(|each| each == "--memory") {
        workspace::memory()
    } else {
        let mut every = workspace::measured();
        every.extend(use_case::measured());
        every
    };
    measure::print(&measured);
}
