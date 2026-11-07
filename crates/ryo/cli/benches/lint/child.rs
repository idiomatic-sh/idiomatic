//! The measuring processes the rows spawn (`--measure <set>`, `--measure-cache-phase <label> <directory>`): each runs
//! one thing in a process of its own, so its peak memory is its own, and prints one tab-separated line for the parent
//! to read; and the smoke run of a bench run as a test.

use crate::{base, counting, phases};
use std::time::Instant;

/// The peak memory of this process so far, in megabytes.
fn peak_megabytes() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status")
        .expect("peak memory measurement requires Linux /proc/self/status");
    let kilobytes = status
        .lines()
        .find_map(|line| {
            line.strip_prefix("VmHWM:")?
                .trim()
                .strip_suffix("kB")?
                .trim()
                .parse::<u64>()
                .ok()
        })
        .expect("Linux /proc/self/status reports numeric VmHWM kilobytes");
    kilobytes / 1024
}

/// The prototype's eight rules.
const SPIKED: [&str; 8] = [
    "deep-nesting",
    "glob-imports",
    "empty-else",
    "allow-attributes-per-file",
    "deep-function-nesting",
    "match-on-bool",
    "file-length",
    "duplicate-function-bodies",
];

/// Runs one set of rules over ruff in this process, cold, and prints one line: the set, how many
/// of its rules ran, rows, wall, CPU and peak memory.
pub(crate) fn measure(set: &str) {
    let (names, wanted): (Vec<String>, Vec<_>) = counting::runnable()
        .into_iter()
        .filter(|(name, _)| {
            set == "all"
                || set.split(',').any(|each| each == name)
                || (set == "spike" && SPIKED.contains(&name.as_str()))
        })
        .unzip();
    let ran = wanted.len();
    assert!(ran > 0, "the requested rule set contains runnable rules");
    let started = Instant::now();
    let cpu = cpu_time::ProcessTime::now();
    let rows = phases::lint_ruff(wanted);
    println!(
        "{set}\t{ran}\t{}\t{rows}\t{}\t{}\t{}",
        names.join(","),
        started.elapsed().as_millis(),
        cpu.elapsed().as_millis(),
        peak_megabytes()
    );
}

/// Runs one cache row in this process, over the copy of ruff at `directory`, and prints its line with the peak memory of this process alone.
pub(crate) fn measure_cache_phase(label: &str, directory: &str) {
    let rules: Vec<_> = counting::runnable()
        .into_iter()
        .map(|(_, rule)| rule)
        .collect();
    let phase = phases::ruff_phase(
        std::path::Path::new(directory),
        rules,
        label != "plain",
        match label {
            "plain" => "plain",
            "cold" => "cold",
            "warm" => "warm",
            "edit" => "edit",
            "body" => "body",
            _ => "rename",
        },
    );
    assert_eq!(
        phase.unread_rust, 0,
        "a run over ruff left a Rust file unread ({} reads of the project)",
        phase.projects
    );
    println!("evidence\t{}", phase.output_digest);
    println!(
        "cache\t{}\t{}\t{}\t{}\t{}\t{}",
        phase.label,
        phase.wall,
        phase.cpu,
        phase.reads,
        phase.rows,
        peak_megabytes()
    );
}

/// Every recorded rule that runs, over the small sample: the bench proves it can be run, as a test of it.
pub(crate) fn smoke() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample");
    let rules = base::recordings()
        .iter()
        .filter(|each| each.pending == base::Pending::Missing)
        .map(base::rule_of)
        .collect();
    let (rows, unread) =
        base::lint_over(&root, rules).expect("the recorded rules run over the sample");
    eprintln!(
        "smoke: the recorded rules gave {} rows over the sample, {} files unread",
        rows.len(),
        unread.len()
    );
}
