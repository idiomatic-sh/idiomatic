//! ryo's perf entry point: what ryo costs over ruff, cold, warm, after an edit and in the store it leaves, on the pinned
//! ruff. It prints one Bencher Cost Format document on stdout (`lint/bmf.rs` says which metrics and how they are
//! taken), and what a person reads (the rows of each set of rules, the rounds of each metric) on stderr; Asymptote's
//! `perf` group judges the document against the ratchet, and this bench asserts no budget.
//!
//! Run it with `cargo bench --locked -p idiomatic-sh-ryo --bench lint` on a quiet machine, never in a batch (a full run
//! takes several minutes; `--rounds <n>` takes the cache rows over n rounds, a seed takes six or more). Run as a test
//! (`cargo test --all-targets` passes no `--bench`), it proves the workloads run on a small tree and measures nothing.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a bench that cannot run its recording has nothing to measure: the panic is the failure"
)]
#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a bench's output is what it prints: the cost document on stdout for Asymptote, and the rounds and rows a person reads on stderr"
)]

// The allocator the command line ships, so the bench measures what ships.
/// The allocator on Windows: mimalloc, as ruff and uv ship it.
#[cfg(target_os = "windows")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// The allocator wherever jemalloc builds: jemalloc, as ruff and uv ship it.
#[cfg(all(
    not(target_os = "windows"),
    not(target_os = "openbsd"),
    not(target_os = "aix"),
    not(target_os = "android"),
    any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "powerpc64",
        target_arch = "riscv64"
    )
))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[path = "../tests/fixtures/replay/base.rs"]
mod base;
#[path = "lint/bmf.rs"]
mod bmf;
#[path = "lint/cache.rs"]
mod cache;
#[path = "lint/child.rs"]
mod child;
#[path = "lint/cold.rs"]
mod cold;
#[path = "../tests/fixtures/replay/counting.rs"]
mod counting;
#[path = "../tests/fixtures/replay/phases.rs"]
mod phases;
#[path = "lint/rows.rs"]
mod rows;
#[path = "../tests/fixtures/replay/ruff.rs"]
mod ruff;
#[path = "lint/store.rs"]
mod store;

fn main() {
    let arguments: Vec<String> = std::env::args().collect();
    if let Some(at) = arguments
        .iter()
        .position(|each| each == "--measure-cache-phase")
    {
        if let (Some(label), Some(directory)) = (arguments.get(at + 1), arguments.get(at + 2)) {
            child::measure_cache_phase(label, directory);
        }
        return;
    }
    if let Some(set) = arguments
        .iter()
        .position(|each| each == "--measure")
        .and_then(|at| arguments.get(at + 1))
    {
        child::measure(set);
        return;
    }
    // Run as a test (`cargo test --all-targets` passes no `--bench`), it proves the workloads run on a small tree and measures nothing.
    if !arguments.iter().any(|each| each == "--bench") {
        child::smoke();
        return;
    }
    let rounds = arguments
        .iter()
        .position(|each| each == "--rounds")
        .map_or(bmf::ROUNDS, |at| {
            arguments
                .get(at + 1)
                .expect("--rounds requires a positive integer")
                .parse::<usize>()
                .expect("--rounds requires a positive integer")
        });
    assert!(rounds > 0, "--rounds requires a positive integer");
    if arguments.iter().any(|each| each == "--diagnostic-rules") {
        rows::sets();
        return;
    }
    let declared = base::recordings();
    assert!(
        !declared.is_empty()
            && declared
                .iter()
                .all(|rule| rule.pending == base::Pending::Missing),
        "benchmark requires every configured recorded rule; pending rules cannot be omitted"
    );
    let expected: std::collections::BTreeSet<_> =
        declared.iter().map(|rule| rule.name.as_str()).collect();
    let runnable = counting::runnable();
    let actual: std::collections::BTreeSet<_> =
        runnable.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(expected.len(), declared.len(), "duplicate declared rules");
    assert_eq!(actual.len(), runnable.len(), "duplicate runnable rules");
    assert_eq!(actual, expected, "benchmark must run every configured rule");
    let mut metrics = cold::every_rule();
    metrics.extend(cache::of_ruff(rounds));
    metrics.extend(store::of_each_target());
    bmf::print(&metrics);
}
