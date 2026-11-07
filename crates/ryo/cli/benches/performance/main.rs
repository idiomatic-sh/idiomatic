//! Ryo's command perf entry point: every command's workload, one module per resource, each taken for its rounds. It
//! prints one Bencher Metric Format document on stdout (benchmark = `<resource>/<command>`, measure = `cpu-ms`, the
//! process's own CPU time: a busy host moves it far less than wall time); every workload runs with `-q` so that stdout
//! carries nothing else. Asymptote's `perf` group judges it against the ratchet, the bench asserts no budget. A setup or
//! a run that fails panics, which fails the bench, as does a workload with no rounds or an unwritable stdout. What ryo
//! costs over ruff is the other entry, `benches/lint.rs`.
//!
//! Run it with `cargo bench --locked -p idiomatic-sh-ryo --bench commands`.

#![expect(
    clippy::expect_used,
    reason = "a bench that cannot run its query has nothing to measure: the panic is the failure"
)]

mod measure;
mod operator;
mod query;
mod relation;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut measured = operator::measured();
    measured.extend(query::measured());
    measured.extend(relation::measured());
    measure::print(&measured)
}
